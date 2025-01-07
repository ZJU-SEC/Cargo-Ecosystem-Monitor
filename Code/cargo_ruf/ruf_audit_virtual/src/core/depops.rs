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
    fn resolve_condrufs<'ctx>(
        &self,
        resolve: &Resolve,
        name: &str,
        ver: &str,
        condrufs: &'ctx CondRufs,
    ) -> Result<Vec<&'ctx String>, AuditError>;
    /// Check if the rufs are usable, and return the failed rufs.
    fn filter_rufs<'ctx>(&self, rustv: u32, rufs: Vec<&'ctx String>) -> Vec<&'ctx String>;

    /// First time resolve
    fn first_resolve(&self) -> Result<(Resolve, Tree), AuditError>;
    /// Update resolve accordingsly
    fn update_resolve(
        &self,
        prev_resolve: &Resolve,
        updates: Vec<(String, String, String)>,
    ) -> Result<(Resolve, Tree), AuditError>;
    /// Get lockfile from the resolve
    fn get_resolve_lockfile(&self, resolve: &Resolve) -> Result<String, AuditError>;
}
