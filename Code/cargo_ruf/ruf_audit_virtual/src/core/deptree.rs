use std::{cell::RefCell, io::Write, rc::Rc};

use cargo::core::Resolve;
use cargo_lock::dependency::{
    graph::{EdgeDirection, Graph, NodeIndex},
    Tree,
};
use fxhash::{FxHashMap, FxHashSet};
use petgraph::visit::EdgeRef;
use semver::{Version, VersionReq};

use crate::{
    basic::CondRufs,
    core::{depops::DepOps, error::AuditError},
};

pub type UsedRufs = FxHashMap<String, Vec<String>>;

/// Record and manage the dependency tree of a crate
pub struct DepTreeManager<D: DepOps> {
    /// Rustc versions.
    rustv: u32,

    /// Depencency operators
    depops: D,
    /// Dependency resolve related info
    depresolve: Rc<(Resolve, Tree, UsedRufs)>,
    /// Store the max resolve tree.
    maxresolve: Rc<(Resolve, Tree, UsedRufs)>,

    locals: FxHashSet<String>,

    limited_candidates: RefCell<
        FxHashMap<
            String,
            (
                FxHashSet<Version>,
                FxHashMap<Version, (CondRufs, FxHashMap<String, VersionReq>)>,
            ),
        >,
    >,
    limited_fix: RefCell<FxHashMap<String, VersionReq>>,
}

impl<D: DepOps> DepTreeManager<D> {
    /// Create new DepTreeManager from current configurations.
    pub fn new(ops: D, rustv: u32) -> Result<Self, AuditError> {
        let (resolve, tree) = ops.first_resolve()?;
        let used_rufs = ops.extract_rufs(&resolve)?;
        let resolve = Rc::new((resolve, tree, used_rufs));

        Ok(Self {
            rustv: rustv,

            depops: ops,
            depresolve: resolve.clone(),
            maxresolve: resolve,

            locals: FxHashSet::default(),

            limited_candidates: RefCell::new(FxHashMap::default()),
            limited_fix: RefCell::new(FxHashMap::default()),
        })
    }

    pub fn extract_rufs(&self) -> Result<UsedRufs, AuditError> {
        Ok(self.depresolve.2.clone())
    }

    pub fn filter_rufs<'ctx>(&self, rufs: Vec<&'ctx String>) -> Vec<&'ctx String> {
        self.depops.filter_rufs(self.rustv, rufs)
    }

    pub fn get_graph(&self) -> &Graph {
        self.depresolve.1.graph()
    }

    pub fn set_local(&mut self, nx: &NodeIndex) {
        let node = &self.get_graph()[*nx];
        self.locals
            .insert(format!("{}@{}", node.name, node.version));
    }

    pub fn get_root(&self) -> NodeIndex {
        let roots = self.depresolve.1.roots();
        assert!(roots.len() == 1, "Fatal, multiple roots found");
        roots[0]
    }

    pub fn is_local(&self, nx: &NodeIndex) -> bool {
        let node = &self.get_graph()[*nx];
        self.locals
            .contains(&format!("{}@{}", node.name, node.version))
    }

    pub fn get_lockfile(&self) -> Result<String, AuditError> {
        self.depops.get_resolve_lockfile(&self.depresolve.0)
    }

    /// Update packages in the dependency tree.
    pub fn update_pkgs(
        &mut self,
        updates: Vec<(String, String, String)>,
        debugger: &mut impl Write,
    ) -> Result<(), AuditError> {
        // Updates limits on fix.
        let mut limited_fix_mut = self.limited_fix.borrow_mut();
        for (name, _, fix_ver) in &updates {
            let req = VersionReq::parse(&format!("<={fix_ver}")).unwrap();
            limited_fix_mut.insert(name.clone(), req);
        }
        writeln!(
            debugger,
            "[Deptree Debug] update_pkgs: updates limited_fix {:?}",
            limited_fix_mut
                .iter()
                .map(|(k, req)| format!("{}: {}", k, req))
                .collect::<Vec<_>>()
        )
        .unwrap();

        let (resolve, tree) = self.depops.update_resolve(&self.depresolve.0, updates)?;
        let used_rufs = self.depops.extract_rufs(&resolve)?;

        self.depresolve = Rc::new((resolve, tree, used_rufs));

        Ok(())
    }

    /// Update rust version configs.
    pub fn update_rustv(&mut self, rustv: u32) {
        self.rustv = rustv;
        // Restore the max tree and fix limitations.
        self.depresolve = self.maxresolve.clone();
        self.limited_fix.borrow_mut().clear();
    }

    /// This function will check whether the issue is fixable under current configs.
    pub fn issue_fixable(
        &self,
        issue_nx: NodeIndex,
        debugger: &mut impl Write,
    ) -> Result<FxHashMap<NodeIndex, Version>, AuditError> {
        let graph = self.get_graph();
        let dep = &graph[issue_nx];
        let dep_name = dep.name.to_string();
        let dep_ver = dep.version.to_string();

        writeln!(
            debugger,
            "[Deptree Debug] issue_fixable: check {}@{} fixibility",
            dep_name, dep_ver
        )
        .unwrap();

        // If local, no version fix of course.
        if self.is_local(&issue_nx) {
            return Err(AuditError::FunctionError(
                Some("local crate has no candidates".to_string()),
                Some(issue_nx),
            ));
        }

        // Prepare candidates.
        self.prepare_limited_candidates(issue_nx, debugger)?;

        let limited_candidates_borrow = self.limited_candidates.borrow();

        let mut fix = FxHashMap::default();
        // 1. Check direct fixable first.
        let candidates = &limited_candidates_borrow.get(&dep_name).unwrap().1;
        if candidates.is_empty() {
            return Err(AuditError::InnerError(format!(
                "no candidates found for {}, maybe db errors",
                dep_name
            )));
        }

        let limits_on_candidates = self.limited_fix.borrow().get(&dep_name).cloned();
        let candidates = candidates
            .into_iter()
            .filter(|(v, _)| Self::limited_fix_filter(v, &limits_on_candidates))
            .map(|(k, v)| (k, &v.0));

        let ruf_ok_candidates = self.get_ruf_ok_candidates(&dep_name, &dep_ver, candidates)?;
        writeln!(
            debugger,
            "[Deptree Debug] issue_fixable: direct check, ruf_ok_candidates {:?}",
            ruf_ok_candidates
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
        )
        .unwrap();

        // And here we check whether these ruf-oks are acceptable by parents.
        let req_ok_candidates = self.get_req_ok_candidates(issue_nx, &ruf_ok_candidates)?;
        writeln!(
            debugger,
            "[Deptree Debug] issue_fixable: direct check, req_ok_candidates {:?}",
            req_ok_candidates
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
        )
        .unwrap();
        if !req_ok_candidates.is_empty() {
            // Ok we have usable versions here.
            fix.insert(issue_nx, req_ok_candidates[0].clone());
            return Ok(fix);
        }

        // 2. Or we have to check the parents.
        // The main idea is to find usable parents that accept the ruf-ok childs.
        let ruf_ok_candidates = ruf_ok_candidates
            .into_iter()
            .map(|v| Some(v))
            .chain(vec![None]);
        for usable_child in ruf_ok_candidates {
            if let Ok(mut chain) = self.get_req_ok_parents(issue_nx, usable_child, debugger) {
                if let Some(child) = usable_child {
                    chain.push((issue_nx, child.clone()));
                }
                writeln!(
                    debugger,
                    "[Deptree Debug] issue_fixable: found usable chain {:?}",
                    chain
                        .iter()
                        .map(|(nx, ver)| format!(
                            "{}@{} -> {}",
                            graph[*nx].name,
                            graph[*nx].version.to_string(),
                            ver
                        ))
                        .collect::<Vec<_>>()
                )
                .unwrap();

                let mut check_dup = None;
                for (p, ver) in chain {
                    check_dup = fix.insert(p, ver);
                    if check_dup.is_some() {
                        break;
                    }
                }

                if check_dup.is_some() {
                    // This can be a really complex issue, currently we ignore it.
                    writeln!(debugger,
                        "[Deptree Debug] issue_fixable: multiple fix on parent found when choose child {}@{}",
                        dep_name, usable_child.map(|v| v.to_string()).unwrap_or("None".to_string())
                    ).unwrap();

                    fix.clear();
                    continue;
                } else {
                    return Ok(fix);
                }
            } else {
                writeln!(
                    debugger,
                    "[Deptree Debug] issue_fixable: parent chain check, chain empty when choose child {}@{}",
                    dep_name, usable_child.map(|v| v.to_string()).unwrap_or("None".to_string())
                )
                .unwrap();
            }
        }

        return Err(AuditError::FunctionError(
            Some("no usable parent chain found".to_string()),
            Some(issue_nx),
        ));
    }

    /// Find candidates free from ruf issues under current configs, the returned candidates are sorted by version.
    fn get_ruf_ok_candidates<'ctx>(
        &self,
        pkg_name: &str,
        pkg_ver: &str,
        candidates: impl Iterator<Item = (&'ctx Version, &'ctx CondRufs)>,
    ) -> Result<Vec<&'ctx Version>, AuditError> {
        let mut usable = Vec::new();
        for (ver, condrufs) in candidates.into_iter() {
            let rufs =
                self.depops
                    .resolve_condrufs(&self.depresolve.0, &pkg_name, &pkg_ver, &condrufs)?;
            let issue_rufs = self.depops.filter_rufs(self.rustv, rufs);

            if issue_rufs.is_empty() {
                usable.push(ver);
            }
        }

        // Sort the usable from latest to oldest.
        usable.sort();
        usable.reverse();

        Ok(usable)
    }

    /// Find candidates match parents' version req under current configs.
    fn get_req_ok_candidates<'ctx>(
        &self,
        pkg_nx: NodeIndex,
        candidates: &Vec<&'ctx Version>,
    ) -> Result<Vec<&'ctx Version>, AuditError> {
        let graph = self.get_graph();
        let pkg_name = graph[pkg_nx].name.as_str();
        let parents = self.get_parents(pkg_nx);
        let limited_candidates_borrow = self.limited_candidates.borrow();

        // Collect parents' version req on current package.
        let mut version_reqs = Vec::new();
        for p in parents {
            let p_pkg = &graph[p];

            let (_, meta_reqs) = limited_candidates_borrow
                .get(p_pkg.name.as_str())
                .unwrap()
                .1
                .get(&p_pkg.version)
                .unwrap();
            let req = meta_reqs
                .get(pkg_name)
                .expect("Fatal, cannot find dependency in parent package");

            version_reqs.push((p, req));
        }

        let usable = candidates
            .into_iter()
            .filter(|ver| version_reqs.iter().all(|(_, req)| req.matches(ver)))
            .map(|ver| *ver)
            .collect();

        Ok(usable)
    }

    /// Get max usable parents version chain in tree that accept the given child version.
    fn get_req_ok_parents(
        &self,
        child_nx: NodeIndex,
        child: Option<&Version>,
        debugger: &mut impl Write,
    ) -> Result<Vec<(NodeIndex, Version)>, AuditError> {
        let parents = self.get_parents(child_nx);
        let limited_candidates_borrow = self.limited_candidates.borrow();
        let graph = self.get_graph();
        let mut fix = Vec::new();

        for p in parents {
            writeln!(
                debugger,
                "[Deptree Debug] get_req_ok_parents: check parent {}@{} - child {}@{}",
                graph[p].name,
                graph[p].version,
                graph[child_nx].name,
                child.map(|v| v.to_string()).unwrap_or("None".to_string())
            )
            .unwrap();
            if self.is_local(&p) {
                // local reached, we check whether this child is acceptable or not.
                if child.is_none() {
                    // Of course locals cannot remove this child.
                    return Err(AuditError::FunctionError(None, None));
                }

                let (_, meta_reqs) = limited_candidates_borrow
                    .get(graph[p].name.as_str())
                    .unwrap()
                    .1
                    .get(&graph[p].version)
                    .unwrap();

                let req = meta_reqs
                    .get(graph[child_nx].name.as_str())
                    .expect("Fatal, cannot find dependency in parent package");

                if req.matches(child.unwrap()) {
                    return Ok(fix);
                } else {
                    return Err(AuditError::FunctionError(None, None));
                }
            }

            let req_ok_parents = self.get_req_ok_parent(p, child_nx, child)?;
            writeln!(
                debugger,
                "[Deptree Debug] get_req_ok_parents: parent {} req_ok {:?}",
                graph[p].name,
                req_ok_parents
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
            )
            .unwrap();
            let usables = self.get_req_ok_candidates(p, &req_ok_parents.iter().collect())?;
            writeln!(
                debugger,
                "[Deptree Debug] get_req_ok_parents: parent {} usables {:?}",
                graph[p].name,
                usables.iter().map(|v| v.to_string()).collect::<Vec<_>>()
            )
            .unwrap();
            if !usables.is_empty() {
                // Ok we find needed parents.
                fix.push((p, usables[0].clone()));
            } else {
                // Or we still need to go up, try from latest req_ok_parents.
                for req_ok_p in req_ok_parents {
                    if let Ok(chain) = self.get_req_ok_parents(p, Some(&req_ok_p), debugger) {
                        // writeln!(
                        //     debugger,
                        //     "[Deptree Debug] get_req_ok_parents: parent {} chain {:?}",
                        //     graph[p].name,
                        //     chain
                        //         .iter()
                        //         .map(|(nx, ver)| format!("{}-{}", graph[*nx].name, ver))
                        //         .collect::<Vec<_>>()
                        // )
                        // .unwrap();
                        fix.extend(chain);
                        fix.push((p, req_ok_p));
                        break;
                    }
                }
                if fix.is_empty() {
                    // No usable parents found, the fix failed.
                    return Err(AuditError::FunctionError(None, None));
                }
            }
        }

        Ok(fix)
    }

    /// Get all usable versions of one parent.
    fn get_req_ok_parent(
        &self,
        parent_nx: NodeIndex,
        child_nx: NodeIndex,
        child: Option<&Version>,
    ) -> Result<Vec<Version>, AuditError> {
        let graph = self.get_graph();
        let parent_pkg = &graph[parent_nx];
        let parent_name = parent_pkg.name.as_str();
        let parent_ver = parent_pkg.version.to_string();

        let child_pkg = &graph[child_nx];
        let child_name = child_pkg.name.as_str();

        let limited_candidates_borrow = self.limited_candidates.borrow();

        let parent_candidates = &limited_candidates_borrow.get(parent_name).unwrap().1;
        let limits_on_candidates = self.limited_fix.borrow().get(parent_name).cloned();
        let parent_candidates_iter = parent_candidates
            .into_iter()
            .filter(|(v, _)| Self::limited_fix_filter(v, &limits_on_candidates))
            .map(|(k, v)| (k, &v.0));

        let ruf_ok_candidates =
            self.get_ruf_ok_candidates(&parent_name, &parent_ver, parent_candidates_iter)?;

        let mut usable = Vec::new();
        for p in ruf_ok_candidates {
            let (_, meta_reqs) = parent_candidates.get(&p).unwrap();
            if let Some(child) = child {
                if let Some(req) = meta_reqs.get(child_name) {
                    if req.matches(child) {
                        usable.push(p.clone());
                    }
                } else {
                    // The parent nolonger need this child dep, so ok.
                    usable.push(p.clone());
                }
            } else {
                if meta_reqs.get(child_name).is_none() {
                    // Here we only want nonreq parents.
                    usable.push(p.clone());
                }
            }
        }

        Ok(usable)
    }

    /// Get the parents of a node in the dependency tree.
    fn get_parents(&self, depnx: NodeIndex) -> Vec<NodeIndex> {
        self.depresolve
            .1
            .graph()
            .edges_directed(depnx, EdgeDirection::Incoming)
            .map(|edge| edge.source())
            .collect()
    }

    /// Focus on possible candiates, not all candidates.
    fn prepare_limited_candidates(
        &self,
        pkg_nx: NodeIndex,
        debugger: &mut impl Write,
    ) -> Result<(), AuditError> {
        let graph = self.get_graph();
        let pkg = &graph[pkg_nx];
        let pkg_name = pkg.name.to_string();

        assert!(
            !self.is_local(&pkg_nx),
            "Fatal, cannot prepare local candidates"
        );

        if self
            .limited_candidates
            .borrow()
            .get(pkg.name.as_str())
            .is_some_and(|info| info.0.get(&pkg.version).is_some())
        {
            writeln!(
                debugger,
                "[Deptree Debug] prepare_limited_candidates: prepare candidates {}@{}, it's already done.",
                pkg_name, pkg.version
            )
            .unwrap();
            return Ok(());
        }

        // writeln!(
        //     debugger,
        //     "[Deptree Debug] prepare_limited_candidates: prepare candidates {}.",
        //     pkg_name
        // )
        // .unwrap();

        // Or we got to prepare it, along with all its parents, up to the locals.
        let mut possible_candidates = self.depops.get_all_candidates(&pkg_name)?;
        let parents = self.get_parents(pkg_nx);
        for p in parents {
            let reqs = self
                .prepare_limited_parents(p, pkg_nx, debugger)?
                .into_iter()
                .collect::<FxHashSet<VersionReq>>();

            possible_candidates = possible_candidates
                .into_iter()
                .filter(|(k, _)| reqs.iter().any(|req| req.matches(k)))
                .collect();

            writeln!(
                debugger,
                "[Deptree Debug] prepare_limited_candidates: parent {} reqs {:?}.",
                graph[p].name,
                reqs.iter().map(|req| req.to_string()).collect::<Vec<_>>(),
            )
            .unwrap();
        }

        let mut limited_candidates_borrow_mut = self.limited_candidates.borrow_mut();
        let entry = limited_candidates_borrow_mut
            .entry(pkg_name.clone())
            .or_insert((FxHashSet::default(), FxHashMap::default()));

        let mut datas = FxHashMap::default();
        for (candidate, condrufs) in possible_candidates {
            let meta_reqs = match self
                .depops
                .get_pkg_versionreq(&pkg_name, &candidate.to_string())
            {
                Ok(reqs) => reqs,
                Err(_e) => {
                    // writeln!(
                    //     debugger,
                    //     "[Deptree Debug] prepare_limited_candidates: {}@{} get reqs failed with error {:?}.",
                    //     pkg_name, candidate, e
                    // )
                    // .unwrap();
                    continue;
                }
            };

            datas.insert(candidate, (condrufs, meta_reqs));
        }

        writeln!(
            debugger,
            "[Deptree Debug] prepare_limited_candidates: possible candidates for {pkg_name} {:?}",
            datas.keys().map(|v| v.to_string()).collect::<Vec<String>>()
        )
        .unwrap();

        entry.0.insert(pkg.version.clone());
        entry.1.extend(datas);

        Ok(())
    }

    fn prepare_limited_parents(
        &self,
        parent_nx: NodeIndex,
        child_nx: NodeIndex,
        debugger: &mut impl Write,
    ) -> Result<Vec<VersionReq>, AuditError> {
        let graph = self.get_graph();
        let parent_pkg = &graph[parent_nx];
        let parent_name = parent_pkg.name.to_string();
        let parent_ver = parent_pkg.version.to_string();
        let child_pkg = &graph[child_nx];

        writeln!(
            debugger,
            "[Deptree Debug] prepare_limited_parents: prepare parents {} for child {}.",
            parent_name, child_pkg.name
        )
        .unwrap();

        if self.is_local(&parent_nx) {
            let meta_reqs = self.depops.get_pkg_versionreq(&parent_name, &parent_ver)?;
            let mut datas = FxHashMap::default();

            let req = meta_reqs
                .get(child_pkg.name.as_str())
                .cloned()
                .expect("Fatal, cannot find dependency in parent package");

            datas.insert(parent_pkg.version.clone(), (CondRufs::empty(), meta_reqs));

            writeln!(
                debugger,
                "[Deptree Debug] prepare_limited_parents: it's local parent {}@{} with req {}.",
                parent_name, parent_ver, req
            )
            .unwrap();

            let mut versions = FxHashSet::default();
            versions.insert(parent_pkg.version.clone());

            self.limited_candidates
                .borrow_mut()
                .insert(parent_name, (versions, datas));

            return Ok(vec![req]);
        }

        if let Some((versions, datas)) = self.limited_candidates.borrow().get(&parent_name) {
            if let Some(_) = versions.get(&parent_pkg.version) {
                // Already prepared.
                let mut all_reqs = Vec::new();

                for (_, (_, meta_reqs)) in datas.iter() {
                    if let Some(req) = meta_reqs.get(child_pkg.name.as_str()) {
                        all_reqs.push(req.clone());
                    } else {
                        all_reqs.push(VersionReq::STAR);
                    }
                }

                writeln!(
                    debugger,
                    "[Deptree Debug] prepare_limited_parents: parent {} already prepared.",
                    parent_name,
                )
                .unwrap();

                return Ok(all_reqs);
            }
        }

        // Prepare the parent candidates.
        let mut possible_parent_candidates = self.depops.get_all_candidates(&parent_name)?;

        let parent_parents = self.get_parents(parent_nx);
        for parent_parent in parent_parents {
            let reqs = self
                .prepare_limited_parents(parent_parent, parent_nx, debugger)?
                .into_iter()
                .collect::<FxHashSet<VersionReq>>();

            possible_parent_candidates = possible_parent_candidates
                .into_iter()
                .filter(|(k, _)| reqs.iter().any(|req| req.matches(&k)))
                .collect();

            writeln!(
                debugger,
                "[Deptree Debug] prepare_limited_parents: parent {}'s parent {} reqs {:?}.",
                parent_name,
                graph[parent_parent].name,
                reqs.iter().map(|req| req.to_string()).collect::<Vec<_>>()
            )
            .unwrap();
        }

        let mut limited_candidates_borrow_mut = self.limited_candidates.borrow_mut();
        let entry = limited_candidates_borrow_mut
            .entry(parent_name.clone())
            .or_insert((FxHashSet::default(), FxHashMap::default()));

        let mut datas = FxHashMap::default();
        let mut all_reqs = Vec::new();
        for (parent_candidate, condrufs) in possible_parent_candidates {
            let meta_reqs = match self
                .depops
                .get_pkg_versionreq(&parent_name, &parent_candidate.to_string())
            {
                Ok(reqs) => reqs,
                Err(_e) => {
                    // writeln!(
                    //     debugger,
                    //     "[Deptree Debug] prepare_limited_parents: parent {}@{} get reqs failed with error {:?}.",
                    //     parent_name, parent_candidate, e
                    // )
                    // .unwrap();
                    continue;
                }
            };

            if let Some(req) = meta_reqs.get(child_pkg.name.as_str()) {
                all_reqs.push(req.clone());
            } else {
                all_reqs.push(VersionReq::STAR);
            }

            datas.insert(parent_candidate, (condrufs, meta_reqs));
        }

        writeln!(
            debugger,
            "[Deptree Debug] prepare_limited_parents: possible parent candidates for {} {:?}",
            parent_name,
            datas.keys().map(|v| v.to_string()).collect::<Vec<String>>()
        )
        .unwrap();

        entry.0.insert(parent_pkg.version.clone());
        entry.1.extend(datas);

        Ok(all_reqs)
    }

    fn limited_fix_filter(v: &Version, limit: &Option<VersionReq>) -> bool {
        if let Some(limit) = limit {
            limit.matches(v)
        } else {
            true
        }
    }
}
