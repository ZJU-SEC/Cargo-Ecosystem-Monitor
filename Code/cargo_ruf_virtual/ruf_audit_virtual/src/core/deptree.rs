use std::cell::RefCell;

use cargo_lock::{
    dependency::{
        graph::{EdgeDirection, Graph, NodeIndex},
        Tree,
    },
    Version,
};
use fxhash::FxHashMap;
use petgraph::visit::EdgeRef;

use crate::core::{depops::DepOps, error::AuditError};

/// Record and manage the dependency tree of a crate
pub struct DepTreeManager<D: DepOps> {
    /// Dependency tree
    deptree: Tree,
    /// Rustc versions.
    rustv: u32,
    /// Depencency operators
    depops: D,
    /// Limit by lists
    limit_by: RefCell<FxHashMap<NodeIndex, NodeIndex>>,
}

impl<D: DepOps> DepTreeManager<D> {
    /// Create new DepTreeManager from current configurations.
    pub fn new(ops: D, rustv: u32) -> Result<Self, AuditError> {
        let deptree = ops.get_deptree()?;

        Ok(Self {
            deptree: deptree,
            rustv: rustv,
            depops: ops,
            limit_by: RefCell::new(FxHashMap::default()),
        })
    }

    pub fn extract_rufs(&self) -> Result<FxHashMap<String, Vec<String>>, AuditError> {
        self.depops.extract_rufs()
    }

    pub fn check_rufs(&self, rufs: &Vec<String>) -> bool {
        self.depops.check_rufs(self.rustv, rufs)
    }

    pub fn get_rustv(&self) -> u32 {
        self.rustv
    }

    pub fn set_rustv(&mut self, rustv: u32) {
        self.rustv = rustv;
    }

    pub fn get_graph(&self) -> &Graph {
        self.deptree.graph()
    }

    pub fn get_root(&self) -> NodeIndex {
        let roots = self.deptree.roots();
        assert!(roots.len() == 1, "Fatal, multiple roots found");
        roots[0]
    }

    /// Get usable candidates of a node that match it's parents' version req, and free from rufs issues.
    pub fn get_candidates(&self, pkgnx: NodeIndex) -> Result<Vec<Version>, AuditError> {
        let graph = self.get_graph();
        let pkg = &graph[pkgnx];

        // if local, no candidates
        let name_ver = format!("{}@{}", pkg.name, pkg.version);
        // FIXME: add local checks
        // if self.locals.contains_key(&name_ver) {
        //     // println!("[Debug - get_candidates] local {name_ver}, no candidates");
        //     return Ok(FxHashMap::default());
        // }

        let parents = self.get_parents(pkgnx);
        assert!(parents.len() >= 1, "Fatal, root has no parents");

        let candidates = self.depops.get_all_candidates(pkg.name.as_str())?;

        if candidates.is_empty() {
            // FIXME: This shall be another kinds of issues.
            unimplemented!()
        }

        // collect version req
        let mut version_reqs = Vec::new();
        for p in parents {
            let p_pkg = &graph[p];
            let meta = self
                .depops
                .get_pkg_versionreq(p_pkg.name.as_str(), p_pkg.version.to_string().as_str())?;
            let req = meta
                .into_iter()
                .find(|(name, _)| name == pkg.name.as_str())
                .expect("Fatal, cannot find dependency in parent package")
                .1;
            // prepare for relaxing strict parents.
            let lowest = candidates
                .keys()
                .filter(|key| req.matches(key))
                .min()
                .cloned()
                .expect("Fatal, cannot find lowest allowing version");
            version_reqs.push((p, req, lowest));
        }

        // We assume parents who restricts the version most is the one not allow min_lowest,
        // and it shall be updated later, if we need up fix.
        // This assumption won't hold for all cases (cases with complex version req),
        // but most of the times it works.
        let min_lowest = version_reqs
            .iter()
            .map(|vr| &vr.2)
            .min()
            .expect("Fatal, no min version found");

        let mut limit_by = None;
        for version_req in version_reqs.iter() {
            if version_req.2 > *min_lowest {
                limit_by = Some(version_req.0);
                break;
            }
        }

        // Add limits
        // FIXME: the limit design shall change.
        self.limit_by
            .borrow_mut()
            .insert(pkgnx, limit_by.expect("Fatal, no strict parent found"));

        // we choose candidates as:
        // 1. match its dependents' version req
        // 2. smaller than current version
        // 3. free from ruf issues
        // we will record who restricts the version most, for later up fix.
        let mut usable = Vec::new();
        for (ver, condrufs) in candidates.into_iter().filter(|(ver, _)| {
            version_reqs
                .iter()
                .all(|(_, req, _)| req.matches(ver) && ver < &pkg.version)
        }) {
            let rufs = self.depops.resolve_condrufs(condrufs)?;
            if self.depops.check_rufs(self.rustv, &rufs) {
                usable.push(ver);
            }
        }

        Ok(usable)
    }

    /// Get the parents of a node in the dependency tree.
    fn get_parents(&self, depnx: NodeIndex) -> Vec<NodeIndex> {
        self.deptree
            .graph()
            .edges_directed(depnx, EdgeDirection::Incoming)
            .map(|edge| edge.source())
            .collect()
    }
}
