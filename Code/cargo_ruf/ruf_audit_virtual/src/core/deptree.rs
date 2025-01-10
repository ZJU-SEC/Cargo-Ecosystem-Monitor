use std::{cell::RefCell, cmp::min, io::Write, rc::Rc};

use cargo::core::Resolve;
use cargo_lock::dependency::{
    graph::{EdgeDirection, Graph, NodeIndex},
    Tree,
};
use fxhash::{FxHashMap, FxHashSet};
use petgraph::{algo::toposort, visit::EdgeRef};
use semver::{Version, VersionReq};

use crate::{
    basic::{CondRuf, CondRufs},
    core::{depops::DepOps, error::AuditError},
};

use super::depops::DepVersionReq;

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
                bool, // Candidates removable
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

    pub fn set_local(&mut self, nx: &NodeIndex) {
        let node = &self.get_graph()[*nx];
        self.locals
            .insert(format!("{}@{}", node.name, node.version));
    }

    pub fn is_local(&self, nx: &NodeIndex) -> bool {
        let node = &self.get_graph()[*nx];
        self.locals
            .contains(&format!("{}@{}", node.name, node.version))
    }

    pub fn get_graph(&self) -> &Graph {
        self.depresolve.1.graph()
    }

    pub fn get_root(&self) -> NodeIndex {
        let roots = self.depresolve.1.roots();
        assert!(roots.len() == 1, "Fatal, multiple roots found");
        roots[0]
    }

    pub fn get_lockfile(&self) -> Result<String, AuditError> {
        self.depops.get_resolve_lockfile(&self.depresolve.0)
    }

    pub fn set_fix_limit(&self, fixes: &Vec<(String, Version, Version)>) {
        // Updates limits on fix, this will also accelerate the step fixing.
        let mut limited_fix_mut = self.limited_fix.borrow_mut();
        for (name, _, fix_ver) in fixes {
            let req = VersionReq::parse(&format!("<={fix_ver}")).unwrap();
            limited_fix_mut.insert(name.clone(), req);
        }
        drop(limited_fix_mut);
    }

    pub fn clear_fix_limit(&self) {
        self.limited_fix.borrow_mut().clear();
    }

    /// Update rust version configs.
    pub fn switch_rustv(&mut self, rustv: u32) {
        self.rustv = rustv;
        // Restore the max tree and fix limitations.
        self.depresolve = self.maxresolve.clone();

        self.limited_candidates.borrow_mut().clear();
        self.limited_fix.borrow_mut().clear();
    }

    /// Fix one issue, step by step.
    pub fn issue_dofix(
        &mut self,
        issue_nx: NodeIndex,
        fixes: Vec<(String, Version, Version)>,
        debugger: &mut impl Write,
    ) -> Result<Vec<(String, Version, Version)>, AuditError> {
        let max_step = fixes.len();
        let mut cur_step = 0;
        let mut do_fix = Vec::new();

        let issue_pkg = self.get_graph()[issue_nx].clone();
        writeln!(
            debugger,
            "[Deptree Debug] issue_dofix: start fixing {}@{} with limits: {:?}",
            issue_pkg.name,
            issue_pkg.version,
            self.limited_fix
                .borrow()
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect::<Vec<_>>()
        )
        .unwrap();

        let mut first_fix = Some(fixes);
        loop {
            assert!(cur_step <= max_step, "Fatal, step fixing exceeds max step");
            let graph = self.get_graph();
            if let Some((_, issue_nx)) = self.depresolve.1.nodes().iter().find(|(_, nx)| {
                graph[**nx].name == issue_pkg.name && graph[**nx].version == issue_pkg.version
            }) {
                let mut step_fixes = if first_fix.is_some() {
                    first_fix.take().unwrap()
                } else {
                    self.get_step_fix(*issue_nx, debugger)?
                        .into_iter()
                        .map(|(nx, fix_ver)| {
                            (
                                graph[nx].name.to_string(),
                                graph[nx].version.clone(),
                                fix_ver,
                            )
                        })
                        .collect::<Vec<_>>()
                };

                writeln!(
                    debugger,
                    "[Deptree Debug] issue_dofix: step fixing {}@{} -> {}, changeable remaining {:?}",
                    step_fixes[0].0,
                    step_fixes[0].1,
                    step_fixes[0].2,
                    step_fixes
                        .iter()
                        .skip(1)
                        .map(|(name, ver, fix_ver)| format!("{}@{} -> {}", name, ver, fix_ver))
                        .collect::<Vec<_>>()
                )
                .unwrap();

                let fix = step_fixes.remove(0);
                do_fix.push(fix.clone());

                let (resolve, tree) = self.depops.update_resolve(&self.depresolve.0, fix)?;
                let used_rufs = self.depops.extract_rufs(&resolve)?;
                self.depresolve = Rc::new((resolve, tree, used_rufs));

                cur_step += 1;
            } else {
                writeln!(
                    debugger,
                    "[Deptree Debug] issue_dofix: issue dep {}@{} already gone.",
                    issue_pkg.name, issue_pkg.version
                )
                .unwrap();
                break;
            }
        }

        Ok(do_fix)
    }

    /// This function will check whether the issue is fixable under current configs.
    pub fn issue_fixable(
        &self,
        issue_nx: NodeIndex,
        debugger: &mut impl Write,
    ) -> Result<Vec<(NodeIndex, Version)>, AuditError> {
        // let graph = self.get_graph();
        // let dep = &graph[issue_nx];
        // writeln!(
        //     debugger,
        //     "[Deptree Debug] issue_fixable: check {}@{} fixibility",
        //     dep.name, dep.version
        // )
        // .unwrap();

        // If local, no version fix of course.
        if self.is_local(&issue_nx) {
            return Err(AuditError::FunctionError(
                Some("local crate has no candidates".to_string()),
                Some(issue_nx),
            ));
        }

        // Prepare candidates.
        self.limited_candidates.borrow_mut().clear();

        self.prepare_limited_candidates(issue_nx, debugger)?;
        self.get_step_fix(issue_nx, debugger)
    }

    /// Get the fixing steps.
    fn get_step_fix(
        &self,
        issue_nx: NodeIndex,
        debugger: &mut impl Write,
    ) -> Result<Vec<(NodeIndex, Version)>, AuditError> {
        // let start = Instant::now();
        let mut fixes = self.get_step_fix_inner(issue_nx, debugger)?;
        let mut topo_fix = Vec::new();

        // Do topo sort here.
        let graph = self.get_graph();
        let sorted = toposort(graph, None).expect("Fatal, topo fail with cycles");
        for node in sorted {
            if let Some(fix) = fixes.remove(&node) {
                topo_fix.push((node, fix));
            }
        }

        // writeln!(
        //     debugger,
        //     "[Deptree Time] get_step_fix: {:?}",
        //     start.elapsed()
        // )
        // .unwrap();

        assert!(fixes.is_empty(), "Fatal, fixes mismatch with deptree.");
        Ok(topo_fix)
    }

    /// Get the fixing steps.
    fn get_step_fix_inner(
        &self,
        issue_nx: NodeIndex,
        debugger: &mut impl Write,
    ) -> Result<FxHashMap<NodeIndex, Version>, AuditError> {
        let graph = self.get_graph();
        let dep = &graph[issue_nx];
        let dep_name = dep.name.to_string();
        let dep_ver = dep.version.to_string();

        let mut fix = FxHashMap::default();
        let limited_candidates_borrow = self.limited_candidates.borrow();

        // 1. Check direct fixable first.
        let (_removable, candidates) = &limited_candidates_borrow.get(&dep_name).unwrap();

        let limits_on_candidates = self.limited_fix.borrow().get(&dep_name).cloned();
        let candidates = candidates
            .into_iter()
            .filter(|(v, _)| Self::limited_fix_filter(v, &limits_on_candidates))
            .map(|(k, v)| (k, &v.0));

        let ruf_ok_candidates = self.get_ruf_ok_candidates(&dep_name, &dep_ver, candidates)?;

        // And here we check whether these ruf-oks are acceptable by parents.
        let req_ok_candidates = self.get_req_ok_candidates(issue_nx, &ruf_ok_candidates)?;
        // writeln!(
        //     debugger,
        //     "[Deptree Debug] issue_fixable: check issue deps:\n ruf ok: {:?}\n req ok: {:?}",
        //     ruf_ok_candidates
        //         .iter()
        //         .map(|v| v.to_string())
        //         .collect::<Vec<_>>(),
        //     req_ok_candidates
        //         .iter()
        //         .map(|v| v.to_string())
        //         .collect::<Vec<_>>()
        // )
        // .unwrap();
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
            // writeln!(
            //     debugger,
            //     "[Deptree Debut] issue_fixable: trying issue dep {} candidates {}",
            //     dep_name,
            //     usable_child
            //         .map(|v| v.to_string())
            //         .unwrap_or("None".to_string())
            // )
            // .unwrap();
            let mut chain = match self.get_req_ok_parents(issue_nx, usable_child, debugger) {
                Ok(chain) => chain,
                Err(e) => {
                    if e.is_inner() {
                        return Err(e);
                    } else {
                        continue;
                    }
                }
            };

            if let Some(child) = usable_child {
                chain.push((issue_nx, child.clone()));
            }

            let mut incompatible_update = false;
            for (p, ver) in chain {
                assert!(graph[p].version != ver, "Fatal, try update the same node");
                if let Some(old_ver) = fix.get(&p) {
                    if check_compatible(old_ver, &ver)? {
                        if old_ver < &ver {
                            fix.insert(p, ver);
                        }
                    } else {
                        writeln!(debugger,
                            "[Deptree Notice] multiple incompatible fix on parent found when choose child {}@{}, incompatible on {} with {} and {}",
                            dep_name, usable_child.map(|v| v.to_string()).unwrap_or("None".to_string()),
                            graph[p].name, old_ver, ver,
                        ).unwrap();
                        incompatible_update = true;
                        break;
                    }
                } else {
                    fix.insert(p, ver);
                }
            }

            if incompatible_update {
                // This can be a really complex issue, currently we ignore it.
                fix.clear();
                continue;
            } else {
                return Ok(fix);
            }
        }

        fn check_compatible(v1: &Version, v2: &Version) -> Result<bool, AuditError> {
            let min = min(v1, v2);
            let req = VersionReq::parse(&format!("^{min}"))
                .map_err(|e| AuditError::InnerError(e.to_string()))?;
            return Ok(req.matches(v1) && req.matches(v2));
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
        let limited_candidates_borrow = self.limited_candidates.borrow();

        // Collect parents' version req on current package.
        let mut version_reqs = Vec::new();
        let parents = self.get_parents_sorted(pkg_nx);
        for p in parents {
            let p_pkg = &graph[p];

            let req = match limited_candidates_borrow
                .get(p_pkg.name.as_str())
                .and_then(|(_removable, candidates)| candidates.get(&p_pkg.version))
                .map(|(_, meta_reqs)| {
                    meta_reqs
                        .get(pkg_name)
                        .expect("Fatal, cannot find dependency in parent package")
                        .clone()
                }) {
                Some(req) => req,
                None => self
                    .depops
                    .get_pkg_versionreq(p_pkg.name.as_str(), &p_pkg.version.to_string())?
                    .get(pkg_name)
                    .expect("Fatal, cannot find dependency in parent package")
                    .clone(),
            };

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
        let parents = self.get_parents_sorted(child_nx);
        let graph = self.get_graph();
        let mut fixes = Vec::new();

        let limited_candidates_borrow = self.limited_candidates.borrow();

        for p in parents {
            if let Some(child) = child {
                let p_pkg = &graph[p];
                let child_pkg = &graph[child_nx];

                let req = limited_candidates_borrow
                    .get(p_pkg.name.as_str())
                    .and_then(|(_removable, candidates)| candidates.get(&p_pkg.version))
                    .map(|(_, meta_reqs)| {
                        meta_reqs
                            .get(child_pkg.name.as_str())
                            .expect("Fatal, cannot find dependency in parent package")
                            .clone()
                    })
                    .unwrap_or_else(|| {
                        self.depops
                            .get_pkg_versionreq(p_pkg.name.as_str(), &p_pkg.version.to_string())
                            .unwrap()
                            .get(child_pkg.name.as_str())
                            .expect("Fatal, cannot find dependency in parent package")
                            .clone()
                    });
                if req.matches(child) {
                    // Ok this is not the limit parents.
                    continue;
                }
            }

            // writeln!(
            //     debugger,
            //     "[Deptree Debug] get_req_ok_parents: check parent {}@{} - child {}@{}",
            //     graph[p].name,
            //     graph[p].version,
            //     graph[child_nx].name,
            //     child.map(|v| v.to_string()).unwrap_or("None".to_string())
            // )
            // .unwrap();
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
                    continue;
                } else {
                    return Err(AuditError::FunctionError(None, None));
                }
            }

            let req_ok_parents = self.get_req_ok_parent(p, child_nx, child)?;
            let usables = self.get_req_ok_candidates(p, &req_ok_parents.iter().collect())?;
            // writeln!(
            //     debugger,
            //     "[Deptree Debug] get_req_ok_parents: checking parent {}:\n req ok: {:?}\n usable: {:?}",
            //     graph[p].name,
            //     req_ok_parents.iter().map(|v| v.to_string()).collect::<Vec<_>>(),
            //     usables.iter().map(|v| v.to_string()).collect::<Vec<_>>()
            // )
            // .unwrap();
            if !usables.is_empty() {
                // Ok we find needed parents.
                // writeln!(
                //     debugger,
                //     "[Deptree Debug] get_req_ok_parents: found usable parent for {}@{}: {:?}",
                //     graph[child_nx].name,
                //     child.map(|v| v.to_string()).unwrap_or("None".to_string()),
                //     usables[0].to_string(),
                // )
                // .unwrap();
                fixes.push((p, usables[0].clone()));
            } else {
                // Or we still need to go up, try from latest req_ok_parents.
                let mut chain = Vec::new();
                for req_ok_p in req_ok_parents.iter().map(|v| Some(v)).chain(vec![None]) {
                    chain = match self.get_req_ok_parents(p, req_ok_p, debugger) {
                        Ok(chain) => chain,
                        Err(e) => {
                            // writeln!(debugger,
                            //     "[Deptree Debug] get_req_ok_parents: found no usable chain for {}@{}",
                            //     graph[child_nx].name,
                            //     child.map(|v| v.to_string()).unwrap_or("None".to_string())
                            // ).unwrap();
                            if e.is_inner() {
                                return Err(e);
                            } else {
                                continue;
                            }
                        }
                    };
                    if let Some(ok_ver) = req_ok_p {
                        chain.push((p, ok_ver.clone()));
                    }
                    // writeln!(
                    //     debugger,
                    //     "[Deptree Debug] get_req_ok_parents: found usable chain for {}@{}: {:?}",
                    //     graph[child_nx].name,
                    //     child.map(|v| v.to_string()).unwrap_or("None".to_string()),
                    //     chain
                    //         .iter()
                    //         .map(|(x, v)| format!("{}@{}", graph[*x].name, v))
                    //         .collect::<Vec<_>>()
                    // )
                    // .unwrap();
                    break;
                }
                if chain.is_empty() {
                    // No usable parents found, the fix failed.
                    return Err(AuditError::FunctionError(None, None));
                }
                fixes.extend(chain.into_iter());
            }
        }

        Ok(fixes)
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

        // println!(
        //     "[DEBUG] parent_name: {parent_name}, limited_candidates_borrow: {:?}",
        //     limited_candidates_borrow
        //         .iter()
        //         .map(|(name, (_, vers))| format!(
        //             "{} - {:?}\n",
        //             name,
        //             vers.iter().map(|(v, _)| v.to_string()).collect::<Vec<String>>()
        //         ))
        //         .collect::<Vec<String>>()
        // );
        let (_removable, parent_candidates) = &limited_candidates_borrow.get(parent_name).unwrap();

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
                    // NOTICE: If not specify the child to be removed, we won't consider it.
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

    /// Get direct parents, not sorted.
    fn get_direct_parents(&self, depnx: NodeIndex) -> Vec<NodeIndex> {
        self.get_graph()
            .edges_directed(depnx, EdgeDirection::Incoming)
            .map(|edge| edge.source())
            .collect()
    }

    /// Get all parents (up to root), not sorted.
    fn get_all_parents(&self, depnx: NodeIndex) -> FxHashSet<NodeIndex> {
        let graph = self.get_graph();
        let root = self.get_root();

        let mut ancestors = FxHashSet::default();
        let mut stack = vec![depnx];
        while let Some(node) = stack.pop() {
            if ancestors.insert(node) {
                for edge in graph.edges_directed(node, EdgeDirection::Incoming) {
                    stack.push(edge.source());
                }
            }
        }
        assert!(ancestors.contains(&root), "Fatal, root not in ancestors");
        // FIXME: We remove the virt node, shall only used in virt fix.
        ancestors.remove(&root);

        ancestors
    }

    fn get_topo_sort(&self, nodes: FxHashSet<NodeIndex>) -> Vec<NodeIndex> {
        let graph = self.get_graph();
        let sorted = toposort(graph, None).expect("Fatal, topo fail with cycles");

        sorted.into_iter().filter(|n| nodes.contains(n)).collect()
    }

    /// Get parents and sorted by depth.
    fn get_parents_sorted(&self, depnx: NodeIndex) -> Vec<NodeIndex> {
        let parents = self.get_direct_parents(depnx).into_iter().collect();
        self.get_topo_sort(parents)
    }

    /// Focus on possible candiates, not all candidates.
    fn prepare_limited_candidates(
        &self,
        pkg_nx: NodeIndex,
        debugger: &mut impl Write,
    ) -> Result<(), AuditError> {
        // let start = Instant::now();
        let all_parents = self.get_all_parents(pkg_nx);
        let topos = self.get_topo_sort(all_parents);

        // let graph = self.get_graph();
        for nx in topos {
            // if self
            //     .limited_candidates
            //     .borrow()
            //     .get(graph[nx].name.as_str())
            //     .is_some()
            //     && self.check_node_unique(nx, debugger)
            // {
            //     continue;
            // }
            self.prepare_limited_candidates_inner(nx, debugger)?;
        }

        // writeln!(
        //     debugger,
        //     "[Deptree Time] prepare_limited_candidates: {:?}",
        //     start.elapsed()
        // )
        // .unwrap();

        Ok(())
    }

    fn prepare_limited_candidates_inner(
        &self,
        pkg_nx: NodeIndex,
        debugger: &mut impl Write,
    ) -> Result<(), AuditError> {
        let graph = self.get_graph();
        let pkg = &graph[pkg_nx];
        let pkg_name = pkg.name.to_string();
        let pkg_ver = pkg.version.to_string();

        // writeln!(
        //     debugger,
        //     "[Deptree Debug] prepare_limited_candidates_inner: prepare candidates {}@{}.",
        //     pkg_name, pkg_ver
        // )
        // .unwrap();

        // Is it local?
        if self.is_local(&pkg_nx) {
            let meta_reqs = self.depops.get_pkg_versionreq(&pkg_name, &pkg_ver)?;
            let mut datas = FxHashMap::default();

            datas.insert(pkg.version.clone(), (CondRufs::empty(), meta_reqs));

            // writeln!(
            //     debugger,
            //     "[Deptree Debug] prepare_limited_candidates_inner: it's local parent {}@{}.",
            //     pkg_name, pkg_ver
            // )
            // .unwrap();

            self.limited_candidates
                .borrow_mut()
                .insert(pkg_name.clone(), (false, datas));

            return Ok(());
        }

        // Prepare it, along with all its parents.
        let mut possible_candidates = self.depops.get_all_candidates(&pkg_name)?;
        let parents = self.get_direct_parents(pkg_nx);
        let mut removable = Vec::new();
        for p in parents {
            let p_pkg = &graph[p];

            let limited_candidates_borrow = self.limited_candidates.borrow();
            let (p_removable, datas) = limited_candidates_borrow.get(p_pkg.name.as_str()).unwrap();

            if *p_removable {
                // We donot need to filter it.
                removable.push(true);
            } else {
                let mut all_reqs = FxHashSet::default();
                for (_, (_, meta_reqs)) in datas.iter() {
                    if let Some(req) = meta_reqs.get(&pkg_name) {
                        all_reqs.insert(DepVersionReq::from(req));
                    } else {
                        all_reqs.insert(DepVersionReq::Remove);
                        // Early stop.
                        break;
                    }
                }

                if all_reqs.contains(&DepVersionReq::Remove) {
                    // We also donot need to filter.
                    removable.push(true);
                } else {
                    removable.push(false);
                    possible_candidates = possible_candidates
                        .into_iter()
                        .filter(|(k, _)| all_reqs.iter().any(|req| req.matches(k)))
                        .collect();
                }
            }
        }

        if possible_candidates.get(&pkg.version).is_none() {
            // Normally it won't happen, but our version_ruf db may lack infos, and thus cause the parent or the parent's parents
            // not exist. And when this happens, we add current version to the candidates, and set CondRuf to uncond ruf usage.

            // NOTICE: we set the CondRufs to uncond ruf usage, this may amplify the usage of ruf, and cause fixing rate to be lower.
            let used_rufs = self.depops.extract_rufs(&self.depresolve.0)?;
            let rufs = used_rufs
                .get(&pkg_name)
                .cloned()
                .unwrap_or_else(|| Vec::new())
                .into_iter()
                .map(|ruf| CondRuf {
                    cond: None,
                    feature: ruf,
                })
                .collect();

            writeln!(
                debugger,
                "[Deptree Notice] package {}@{} meet with info missing in DB version_ruf.",
                pkg_name, pkg.version
            )
            .unwrap();

            possible_candidates.insert(pkg.version.clone(), CondRufs::new(rufs));
        }

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
                    //     "[Deptree Debug] prepare_limited_candidates_inner: {}@{} get reqs failed with error {:?}.",
                    //     pkg_name, candidate, e
                    // )
                    // .unwrap();
                    continue;
                }
            };

            datas.insert(candidate, (condrufs, meta_reqs));
        }

        // writeln!(
        //     debugger,
        //     "[Deptree Debug] prepare_limited_candidates_inner: possible candidates (X {}) for {pkg_name} {:?}",
        //     removable,
        //     datas.keys().map(|v| v.to_string()).collect::<Vec<String>>()
        // )
        // .unwrap();

        let removable = removable.into_iter().all(|r| r);
        let mut limited_candidates_borrow_mut = self.limited_candidates.borrow_mut();
        let entry = limited_candidates_borrow_mut
            .entry(pkg_name)
            .or_insert((false, datas));

        if removable {
            entry.0 = true;
        }

        return Ok(());
    }

    fn limited_fix_filter(v: &Version, limit: &Option<VersionReq>) -> bool {
        if let Some(limit) = limit {
            limit.matches(v)
        } else {
            true
        }
    }
}
