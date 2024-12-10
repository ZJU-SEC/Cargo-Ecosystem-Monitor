use cargo_lock::{dependency::Tree, Version};
use fxhash::FxHashMap;
use semver::VersionReq;

use super::error::AuditError;
use crate::basic::CondRufs;

pub trait DepOps {
    fn get_all_candidates(&self, name: &str) -> Result<FxHashMap<Version, CondRufs>, AuditError>;
    fn get_pkg_versionreq(
        &self,
        name: &str,
        ver: &str,
    ) -> Result<Vec<(String, VersionReq)>, AuditError>;

    fn get_deptree(&self) -> Result<Tree, AuditError>;

    fn extract_rufs(&self) -> Result<FxHashMap<String, Vec<String>>, AuditError>;
    fn resolve_condrufs(&self, condrufs: CondRufs) -> Result<Vec<String>, AuditError>;
    fn check_rufs(&self, rustv: u32, rufs: &Vec<String>) -> bool;
}
