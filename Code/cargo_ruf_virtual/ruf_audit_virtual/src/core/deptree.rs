use cargo_lock::{
    dependency::{
        graph::{EdgeDirection, NodeIndex},
        Tree,
    },
    Lockfile, Version,
};
use fxhash::FxHashMap;
use petgraph::visit::EdgeRef;
use semver::VersionReq;

use crate::basic::CondRufs;
use crate::core::{depops::DepOps, error::AuditError};

/// Record and manage the dependency tree of a crate
pub struct DepTreeManager<D: DepOps> {
    /// Dependency tree
    deptree: Option<Tree>,
    /// Depencency operators
    depops: D,
}

impl<D: DepOps> DepTreeManager<D> {
    /// Create new DepTreeManager from current configurations.
    pub fn new(ops: D) -> Self {
        Self {
            deptree: None,
            depops: ops,
        }
    }

    pub fn extract_rufs(&self) -> Result<(), AuditError> {
        unimplemented!()
    }

    pub fn check_rufs(&self, rufs: Vec<String>) {
        unimplemented!()
    }

    // /// Get candidates of a node that match it's parents' version req.
    // pub fn get_candidates(
    //     &self,
    //     pkgnx: NodeIndex,
    // ) -> Result<FxHashMap<Version, CondRufs>, AuditError> {
    //     // FIXME: check all related codes again.
    //     let pkg = &self.deptree.graph()[pkgnx];

    //     // if local, no candidates
    //     let name_ver = format!("{}@{}", pkg.name, pkg.version);
    //     if self.locals.contains_key(&name_ver) {
    //         // println!("[Debug - get_candidates] local {name_ver}, no candidates");
    //         return Ok(FxHashMap::default());
    //     }

    //     let parents = self.get_parents(pkgnx);
    //     assert!(parents.len() >= 1, "Fatal, root has no parents");

    //     let candidates = self.depops.get_all_candidates(pkg.name.as_str())?;

    //     // Early return.
    //     if candidates.is_empty() {
    //         return Ok(candidates);
    //     }
    //     // println!(
    //     //     "[Debug - get_candidates] get {name_ver}, candidats: {:?}",
    //     //     candidates.iter().map(|(v, _)| v.to_string()).collect::<Vec<String>>()
    //     // );

    //     // collect version req
    //     let mut version_reqs = Vec::new();
    //     for p in parents {
    //         let p_pkg = &self.deptree.graph()[p];
    //         let meta = self.get_reqs(p_pkg.name.as_str(), p_pkg.version.to_string().as_str())?;
    //         let req = meta
    //             .into_iter()
    //             .find(|(name, _)| name == pkg.name.as_str())
    //             .expect("Fatal, cannot find dependency in parent package")
    //             .1;
    //         // prepare for relaxing strict parents.
    //         let lowest = candidates
    //             .keys()
    //             .filter(|key| req.matches(key))
    //             .min()
    //             .cloned()
    //             .expect("Fatal, cannot find lowest allowing version");
    //         version_reqs.push((p, req, lowest));
    //     }

    //     // We assume parents who restricts the version most is the one not allow min_lowest,
    //     // and it shall be updated later, if we need up fix.
    //     // This assumption won't hold for all cases (cases with complex version req),
    //     // but most of the times it works.
    //     let min_lowest = version_reqs
    //         .iter()
    //         .map(|vr| &vr.2)
    //         .min()
    //         .expect("Fatal, no min version found");

    //     let mut limit_by = None;
    //     for version_req in version_reqs.iter() {
    //         if version_req.2 > *min_lowest {
    //             limit_by = Some(version_req.0);
    //             break;
    //         }
    //     }

    //     // TODO: add limit by to candidates

    //     // we choose candidates as:
    //     // 1. match its dependents' version req
    //     // 2. smaller than current version
    //     // we will record who restricts the version most, for later up fix.
    //     //
    //     // The ruf usability check will be done later, differ from design.
    //     let candidates = candidates
    //         .into_iter()
    //         .filter(|(ver, _)| {
    //             version_reqs
    //                 .iter()
    //                 .all(|(_, req, _)| req.matches(ver) && ver < &pkg.version)
    //         })
    //         .collect();

    //     Ok(candidates)
    // }

    // /// Get the parents of a node in the dependency tree.
    // fn get_parents(&self, depnx: NodeIndex) -> Vec<NodeIndex> {
    //     self.deptree
    //         .graph()
    //         .edges_directed(depnx, EdgeDirection::Incoming)
    //         .map(|edge| edge.source())
    //         .collect()
    // }

    // fn get_reqs(&self, name: &str, ver: &str) -> Result<Vec<(String, VersionReq)>, AuditError> {
    //     self.depops.get_pkg_versionreq(name, ver)
    // }
}
