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

    locals: RefCell<FxHashSet<String>>,
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

            locals: RefCell::new(FxHashSet::default()),
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

    pub fn set_local(&self, nx: &NodeIndex) {
        let node = &self.get_graph()[*nx];
        self.locals
            .borrow_mut()
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
            .borrow()
            .contains(&format!("{}@{}", node.name, node.version))
    }

    pub fn get_lockfile(&self) -> Result<String, AuditError> {
        self.depops.get_resolve_lockfile(&self.depresolve.0)
    }

    /// Update packages in the dependency tree.
    pub fn update_pkg(&mut self, updates: Vec<(String, String, String)>) -> Result<(), AuditError> {
        let (resolve, tree) = self.depops.update_resolve(&self.depresolve.0, updates)?;
        let used_rufs = self.depops.extract_rufs(&resolve)?;

        self.depresolve = Rc::new((resolve, tree, used_rufs));

        Ok(())
    }

    /// Update rust version configs.
    pub fn update_rustv(&mut self, rustv: u32) {
        self.rustv = rustv;
        // Restore the max tree.
        self.depresolve = self.maxresolve.clone();
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

        // If local, no version fix of course.
        if self.is_local(&issue_nx) {
            return Err(AuditError::FunctionError(None, None));
        }

        let mut fix = FxHashMap::default();
        // 1. Check direct fixable first.
        let req = VersionReq::parse("*").map_err(|e| AuditError::InnerError(e.to_string()))?;
        let candidates = self.depops.get_all_candidates(&dep_name, req).unwrap();
        assert!(!candidates.is_empty(), "Fatal, not any candidates found");

        let ruf_ok_candidates = self.get_ruf_ok_candidates(&dep_name, &dep_ver, &candidates)?;
        writeln!(
            debugger,
            "[Deptree Debug] issue_fixable: direct check, ruf_ok_candidates {:?}",
            ruf_ok_candidates
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
        )
        .unwrap();

        // Ok this can not be fixed.
        if ruf_ok_candidates.is_empty() {
            return Err(AuditError::FunctionError(None, None));
        }

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
        for usable_child in ruf_ok_candidates {
            let mut chain = self.get_req_ok_parents(issue_nx, usable_child, debugger)?;
            if !chain.is_empty() {
                chain.push((issue_nx, usable_child.clone()));
                writeln!(
                    debugger,
                    "[Deptree Debug] issue_fixable: parent chain check, chain {:?}",
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
                for (p, ver) in chain {
                    let check_dup = fix.insert(p, ver);
                    assert!(check_dup.is_none(), "Fatal, duplicate parent fix found");
                }

                return Ok(fix);
            } else {
                writeln!(
                    debugger,
                    "[Deptree Debug] issue_fixable: parent chain check, chain empty when choose child {}@{}",
                    dep_name, usable_child
                )
                .unwrap();
            }
        }

        return Err(AuditError::FunctionError(None, None));
    }

    /// Find candidates free from ruf issues under current configs, the returned candidates are sorted by version.
    fn get_ruf_ok_candidates<'ctx>(
        &self,
        pkg_name: &str,
        pkg_ver: &str,
        candidates: &'ctx FxHashMap<Version, CondRufs>,
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

        // Collect parents' version req on current package.
        let mut version_reqs = Vec::new();
        for p in parents {
            let p_pkg = &graph[p];
            let p_name = p_pkg.name.as_str();
            let p_ver = p_pkg.version.to_string();

            let mut meta = self.depops.get_pkg_versionreq(p_name, &p_ver)?;
            let req = meta
                .remove(pkg_name)
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
        child: &Version,
        debugger: &mut impl Write,
    ) -> Result<Vec<(NodeIndex, Version)>, AuditError> {
        let parents = self.get_parents(child_nx);
        let mut fix = Vec::new();

        let graph = self.get_graph();

        for p in parents {
            writeln!(
                debugger,
                "[Deptree Debug] get_req_ok_parents: check parent {} - child {}@{}",
                graph[p].name,
                graph[child_nx].name,
                child.to_string()
            )
            .unwrap();
            if self.is_local(&p) {
                // Ok locals reached, no more version can be changed.
                return Err(AuditError::FunctionError(None, None));
            }

            let req_ok_parents = self.get_req_ok_parent(p, child_nx, child)?;
            writeln!(
                debugger,
                "[Deptree Debug] get_req_ok_parents: parent {} req_ok_parents {:?}",
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
                    if let Ok(chain) = self.get_req_ok_parents(p, &req_ok_p, debugger) {
                        writeln!(
                            debugger,
                            "[Deptree Debug] get_req_ok_parents: parent {} chain {:?}",
                            graph[p].name,
                            chain
                                .iter()
                                .map(|(nx, ver)| format!("{}-{}", graph[*nx].name, ver))
                                .collect::<Vec<_>>()
                        )
                        .unwrap();
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
        child: &Version,
    ) -> Result<Vec<Version>, AuditError> {
        let graph = self.get_graph();
        let parent_pkg = &graph[parent_nx];
        let parent_name = parent_pkg.name.as_str();
        let parent_ver = parent_pkg.version.to_string();

        let child_pkg = &graph[child_nx];
        let child_name = child_pkg.name.as_str();

        let req = VersionReq::parse(&format!("<={}", parent_ver))
            .map_err(|e| AuditError::InnerError(e.to_string()))?;
        let parent_candidates = self.depops.get_all_candidates(parent_name, req)?;
        let ruf_ok_candidates =
            self.get_ruf_ok_candidates(&parent_name, &parent_ver, &parent_candidates)?;
        assert!(
            !ruf_ok_candidates.is_empty(),
            "Fatal, not any parent ruf_ok_candidates found"
        );

        let mut usable = Vec::new();

        for p in ruf_ok_candidates {
            let mut meta = self
                .depops
                .get_pkg_versionreq(parent_name, p.to_string().as_str())?;
            if let Some(req) = meta.remove(child_name) {
                if req.matches(child) {
                    usable.push(p.clone());
                }
            } else {
                // The parent nolonger need this child dep, so ok.
                usable.push(p.clone());
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
}
