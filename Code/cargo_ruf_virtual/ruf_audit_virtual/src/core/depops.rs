use std::sync::Mutex;

use cargo_lock::Version;
use fxhash::FxHashMap;
use postgres::{Client, NoTls};
use semver::VersionReq;

use super::error::AuditError;
use crate::basic::{CondRuf, CondRufs};

pub trait DepOps {
    fn get_all_candidates(&self, name: &str) -> Result<FxHashMap<Version, CondRufs>, AuditError>;
    fn get_pkg_versionreq(
        &self,
        name: &str,
        ver: &str,
    ) -> Result<Vec<(String, VersionReq)>, AuditError>;
}

/// Colect needed info from our databases.
pub struct DepOpsNormal {
    /// For our database connection.
    conn: Mutex<Client>,
}

/*
    -- Currently we HAVE NOT created this table --
    CREATE TABLE version_ruf AS
    SELECT versions.id, versions.num, versions.crate_id, version_feature_ori.conds, version_feature_ori.feature
    FROM versions
    JOIN version_feature_ori
    ON versions.id = version_feature_ori.id

    CREATE VIEW dependencies_with_name AS
    SELECT dependencies.*, crates.name AS crate_name
    FROM dependencies
    JOIN crates ON dependencies.crate_id = crates.id
*/

impl DepOpsNormal {
    pub fn new() -> Self {
        let client = Client::connect(
            "host=localhost dbname=crates user=postgres password=postgres",
            NoTls,
        )
        .unwrap();

        Self {
            conn: Mutex::new(client),
        }
    }

    fn get_crate_id_with_name(&self, crate_name: &str) -> Result<i32, String> {
        let crate_id = self
            .conn
            .lock()
            .unwrap()
            .query(
                "SELECT id FROM crates WHERE name = $1 LIMIT 1",
                &[&crate_name],
            )
            .map_err(|e| e.to_string())?;

        if crate_id.len() == 0 {
            return Err(format!("No crate with name {} found", crate_name));
        }

        Ok(crate_id[0].get::<usize, i32>(0))
    }

    fn get_version_id_with_name_ver(&self, crate_name: &str, version: &str) -> Result<i32, String> {
        let version_id = self
            .conn
            .lock()
            .unwrap()
            .query(
                "SELECT id FROM versions WHERE crate_id = (SELECT id FROM crates WHERE name = $1) AND num = $2 LIMIT 1",
                &[&crate_name, &version],
            )
            .map_err(|e| e.to_string())?;

        if version_id.len() == 0 {
            return Err(format!(
                "No version with namever {}-{} found",
                crate_name, version
            ));
        }

        Ok(version_id[0].get::<usize, i32>(0))
    }

    fn get_rufs_with_crate_id(
        &self,
        crate_id: i32,
    ) -> Result<FxHashMap<Version, CondRufs>, String> {
        let rows = self
            .conn
            .lock()
            .unwrap()
            .query(
                "SELECT * FROM version_ruf WHERE crate_id = $1 ORDER BY id desc",
                &[&crate_id],
            )
            .map_err(|e| e.to_string())?;

        let mut dep_rufs = FxHashMap::default();
        for row in rows {
            let ver: String = row.get(1);
            let ver = Version::parse(&ver)
                .map_err(|e| format!("Version parse failure, invalid version: {} {}", ver, e))?;

            let entry = dep_rufs.entry(ver).or_insert_with(CondRufs::empty);
            if let Some(ruf) = row.get::<_, Option<String>>(4) {
                let cond = row.get::<_, Option<String>>(3);
                let ruf = CondRuf {
                    cond: cond,
                    feature: ruf,
                };

                entry.push(ruf);
            }
        }

        Ok(dep_rufs)
    }

    fn get_reqs_with_version_id(
        &self,
        version_id: i32,
    ) -> Result<Vec<(String, VersionReq)>, String> {
        let rows = self
            .conn
            .lock()
            .unwrap()
            .query(
                "SELECT * FROM dependencies_with_name WHERE version_id = $1 ORDER BY id desc",
                &[&version_id],
            )
            .map_err(|e| e.to_string())?;

        let mut dep_reqs = Vec::new();
        for row in rows {
            let name: String = row.get(9);
            let req: String = row.get(3);
            let req = VersionReq::parse(&req)
                .map_err(|e| format!("VersionReq parse failure, invalid req: {} {}", req, e))?;

            // FIXME: Currently we ignore the optional, feature, target...
            dep_reqs.push((name, req));
        }

        Ok(dep_reqs)
    }
}

impl DepOps for DepOpsNormal {
    /// Here we get maybe usable versions from our database.
    fn get_all_candidates(&self, name: &str) -> Result<FxHashMap<Version, CondRufs>, AuditError> {
        let crate_id = self
            .get_crate_id_with_name(name)
            .map_err(|e| AuditError::InnerError(e))?;

        self.get_rufs_with_crate_id(crate_id)
            .map_err(|e| AuditError::InnerError(e))
    }

    fn get_pkg_versionreq(
        &self,
        name: &str,
        ver: &str,
    ) -> Result<Vec<(String, VersionReq)>, AuditError> {
        let version_id = self
            .get_version_id_with_name_ver(name, ver)
            .map_err(|e| AuditError::InnerError(e))?;

        self.get_reqs_with_version_id(version_id)
            .map_err(|e| AuditError::InnerError(e))
    }
}

#[test]
fn test_depopsnormal() {
    let depops = DepOpsNormal::new();

    let res = depops.get_reqs_with_version_id(600254).unwrap();
    for (v, req) in res {
        println!("{}: {}", v, req);
    }

    let res = depops.get_rufs_with_crate_id(323512).unwrap();
    for (v, rufs) in res {
        println!("{}: {:?}", v, rufs);
    }
}
