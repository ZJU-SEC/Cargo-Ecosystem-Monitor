use cargo::core::Resolve;
use cargo_lock::{dependency::Tree, Version};
use fxhash::FxHashMap;
use semver::VersionReq;

use super::error::AuditError;
use crate::basic::CondRufs;

pub trait DepOps {
    /// Get all candidates of a package.
    fn get_all_candidates(&self, name: &str) -> Result<FxHashMap<Version, CondRufs>, AuditError>;
    /// Get the version requirements of a package.
    fn get_pkg_versionreq(
        &self,
        name: &str,
        ver: &str,
    ) -> Result<FxHashMap<String, VersionReq>, AuditError>;

    /// Extract the rufs from the dependency tree.
    fn extract_rufs(&self, resolve: &Resolve)
        -> Result<FxHashMap<String, Vec<String>>, AuditError>;
    /// Resolve the condrufs to rufs based on current dependency tree.
    fn resolve_condrufs(
        &self,
        resolve: &Resolve,
        name: &str,
        ver: &str,
        condrufs: CondRufs,
    ) -> Result<Vec<String>, AuditError>;
    /// Check if the rufs are usable.
    fn check_rufs(&self, rustv: u32, rufs: &Vec<String>) -> bool;
    /// Similar to the above, but return the failed rufs.
    fn filter_issue_rufs(&self, rustv: u32, rufs: Vec<String>) -> Vec<String>;

    /// First time resolve
    fn first_resolve(&self) -> Result<(Resolve, Tree), AuditError>;
    /// Update resolve accordingsly
    fn update_resolve(
        &self,
        prev_resolve: &Resolve,
        name: &str,
        prev_ver: &str,
        new_ver: &str,
    ) -> Result<(Resolve, Tree), AuditError>;
}
