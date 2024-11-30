use cargo_lock::Version;
use fxhash::FxHashMap;
use semver::VersionReq;

use super::error::AuditError;
use crate::basic::CondRufs;

pub trait DepOps {
    fn get_all_candidates(name: &str) -> Result<FxHashMap<Version, CondRufs>, AuditError>;
    fn get_pkg_versionreq(name: &str, ver: &str) -> Result<Vec<(String, VersionReq)>, AuditError>;
}
