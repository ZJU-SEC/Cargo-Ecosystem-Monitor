use std::env::current_dir;
use std::fs::File;
use std::io::Write;
use std::str::FromStr;
use std::sync::Mutex;

use cargo::core::registry::PackageRegistry;
use cargo::core::resolver::{CliFeatures, HasDevUnits};
use cargo::core::{Resolve, Shell, Workspace};
use cargo::util::interning::InternedString;

use cargo_lock::dependency::Tree;
use cargo_lock::{Lockfile, Version};
use fxhash::{FxHashMap, FxHashSet};
use postgres::{Client, NoTls};
use regex::Regex;
use semver::VersionReq;

use crate::basic::{self, CondRuf, CondRufs};
use crate::core::AuditError;
use crate::core::DepOps;

/*
    -- Currently we HAVE NOT created this table --
    CREATE VIEW version_ruf AS
    SELECT versions_with_name.id, versions_with_name.name, versions_with_name.num, versions_with_name.crate_id, version_feature.conds, version_feature.feature
    FROM versions_with_name
    JOIN version_feature
    ON versions_with_name.id = version_feature.id

    CREATE VIEW dependencies_with_name AS
    SELECT dependencies.*, crates.name AS crate_name
    FROM dependencies
    JOIN crates ON dependencies.crate_id = crates.id
*/

/// Colect needed info from our databases, we call it virtual impl.
/// Used for virtual pipeline analysis.
pub struct DepOpsVirt {
    /// For our database connection.
    conn: Mutex<Client>,

    /// For the target crates.
    name: String,
    ver: String,
    vid: i32,

    /// The local crates.
    locals: FxHashMap<String, Vec<(String, VersionReq)>>,

    /// For the resolve result.
    resolve: Option<Resolve>,
    lockfile: Option<Lockfile>,
}

impl DepOpsVirt {
    pub fn new(name: &str, ver: &str, id: i32) -> Result<Self, AuditError> {
        let client = Client::connect(
            "host=localhost dbname=crates user=postgres password=postgres",
            NoTls,
        )
        .unwrap();

        let mut locals = FxHashMap::default();
        let virt_req = VersionReq::parse(&format!("={}", ver))
            .map_err(|e| AuditError::InnerError(e.to_string()))?;
        locals.insert("virt".to_string(), vec![(name.to_string(), virt_req)]);

        let mut uninit = Self {
            conn: Mutex::new(client),

            name: name.to_string(),
            ver: ver.to_string(),
            vid: id,

            locals: locals,

            resolve: None,
            lockfile: None,
        };

        uninit
            .resolve_current()
            .map_err(|e| AuditError::InnerError(e))?;

        Ok(uninit)
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
                "SELECT id FROM versions_with_name WHERE name = $1 AND num = $2 LIMIT 1",
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

    fn get_cads_with_crate_id(
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

            // FIXME: Currently we DONOT care the optional, feature, target...
            dep_reqs.push((name, req));
        }

        Ok(dep_reqs)
    }

    fn resolve_current(&mut self) -> Result<(), String> {
        let mut features = Vec::new();

        // Create virtual environment.
        // FIXME: Change the tmp home dir.
        let current_path = current_dir().map_err(|e| e.to_string())?;
        let home_path = current_path.join("virt");
        let toml_path = current_path.join("virt.toml");

        if !current_path.exists() {
            std::fs::create_dir_all(&current_path).map_err(|e| e.to_string())?;
        }

        // Get virtual toml file
        let file = self.format_virt_toml_file(&self.name, &self.ver, &features);
        File::create(&toml_path)
            .map_err(|e| e.to_string())?
            .write_all(file.as_bytes())
            .expect("Fatal, write virt.toml file failed");

        // 1. Pre-resolve: get all features first
        let config = cargo::Config::new(Shell::new(), current_path, home_path);
        let ws = Workspace::new(&toml_path, &config).map_err(|e| e.to_string())?;
        let mut registry = PackageRegistry::new(ws.config()).map_err(|e| e.to_string())?;
        let mut resolve = cargo::ops::resolve_with_previous(
            &mut registry,
            &ws,
            &CliFeatures::new_all(true),
            HasDevUnits::No,
            self.resolve.as_ref(),
            None,
            &[],
            true,
        )
        .map_err(|e| e.to_string())?;

        let pkg = resolve
            .query(&format!("{}:{}", &self.name, &self.ver))
            .map_err(|e| e.to_string())?;
        for feature in resolve.summary(pkg).features().keys() {
            features.push(feature.as_str());
        }

        // 2. Resolve with features if found any.
        if !features.is_empty() {
            let file = self.format_virt_toml_file(&self.name, &self.ver, &features);
            File::create(&toml_path)
                .map_err(|e| e.to_string())?
                .write_all(file.as_bytes())
                .expect("Fatal, write virt.toml file failed");

            resolve = cargo::ops::resolve_with_previous(
                &mut registry,
                &ws,
                &CliFeatures::new_all(true),
                HasDevUnits::No,
                Some(&resolve),
                None,
                &[],
                true,
            )
            .map_err(|e| e.to_string())?;
        }

        // And here the resolve is finally usable.
        let lockfile =
            cargo::ops::resolve_to_string(&ws, &mut resolve).map_err(|e| e.to_string())?;
        let lockfile = Lockfile::from_str(&lockfile).map_err(|e| e.to_string())?;

        // Updates the resolve and lockfile.
        self.resolve = Some(resolve);
        self.lockfile = Some(lockfile);

        Ok(())
    }

    fn format_virt_toml_file(&self, name: &str, ver: &str, features: &Vec<&str>) -> String {
        let mut file = String::with_capacity(256); // 预分配足够的空间
        file.push_str(
            "[package]\nname = \"virt\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n",
        );

        // Add all features
        file.push_str(&format!(
            "{} = {{ version = \"={}\", features = [{}] }}",
            name,
            ver,
            features
                .iter()
                .map(|f| format!("\"{}\"", f))
                .collect::<Vec<_>>()
                .join(",")
        ));

        file
    }

    fn extract_rufs_from_current_resolve(&self) -> Result<FxHashMap<String, Vec<String>>, String> {
        let mut rufs = FxHashMap::default();
        let resolve = self.resolve.as_ref().unwrap();
        for pkg_id in resolve.iter() {
            // FIXME: Do we get the used pf correctly or not ?
            let pkg_features = resolve.features(pkg_id);
            let pkg_rufs = self.extract_rufs_from_one_pkg(
                &pkg_id.name().as_str(),
                &pkg_id.version().to_string(),
                pkg_features,
            )?;

            rufs.insert(pkg_id.name().to_string(), pkg_rufs);
        }

        Ok(rufs)
    }

    fn extract_rufs_from_one_pkg(
        &self,
        name: &str,
        ver: &str,
        pkg_feature: &[InternedString],
    ) -> Result<Vec<String>, String> {
        let mut rufs = FxHashSet::default();
        let rows = self
            .conn
            .lock()
            .unwrap()
            .query(
                "SELECT conds, feature FROM version_ruf WHERE name = $1 AND version = $2 and feature != 'no_feature_used'",
                &[&name, &ver],
            )
            .map_err(|e| e.to_string())?;

        for row in rows {
            let cond = row.get::<usize, Option<String>>(0);
            let feature = row.get::<usize, String>(1);

            // Check the conditions and add the feature if enabled.
            if let Some(cond) = cond {
                assert!(!cond.is_empty());
                lazy_static::lazy_static! {
                    static ref RE_CONDS: Regex = Regex::new(r#"feature\s*=\s*"(.*?)""#).unwrap();
                }
                if let Some(caps) = RE_CONDS.captures(&cond) {
                    let cond_pf = caps.get(1).expect("Fatal, invalid regex capture").as_str();
                    if pkg_feature.contains(&InternedString::new(cond_pf)) {
                        rufs.insert(feature);
                    }
                } // Or it's not `feature = "xxx"` condition, we assume it not enabled.
            } else {
                rufs.insert(feature);
            }
        }

        Ok(rufs.drain().collect())
    }
}

impl DepOps for DepOpsVirt {
    fn get_all_candidates(&self, name: &str) -> Result<FxHashMap<Version, CondRufs>, AuditError> {
        // Check locals first
        if self.locals.contains_key(name) {
            return Ok(FxHashMap::default());
        }

        let crate_id = self
            .get_crate_id_with_name(name)
            .map_err(|e| AuditError::InnerError(e))?;

        self.get_cads_with_crate_id(crate_id)
            .map_err(|e| AuditError::InnerError(e))
    }

    fn get_pkg_versionreq(
        &self,
        name: &str,
        ver: &str,
    ) -> Result<Vec<(String, VersionReq)>, AuditError> {
        // Check locals first
        if let Some(localreq) = self.locals.get(name) {
            return Ok(localreq.clone());
        }

        let version_id = self
            .get_version_id_with_name_ver(name, ver)
            .map_err(|e| AuditError::InnerError(e))?;

        self.get_reqs_with_version_id(version_id)
            .map_err(|e| AuditError::InnerError(e))
    }

    fn get_deptree(&self) -> Result<Tree, AuditError> {
        self.lockfile
            .as_ref()
            .unwrap()
            .dependency_tree()
            .map_err(|e| AuditError::InnerError(e.to_string()))
    }

    fn extract_rufs(&self) -> Result<FxHashMap<String, Vec<String>>, AuditError> {
        self.extract_rufs_from_current_resolve()
            .map_err(|e| AuditError::InnerError(e))
    }

    fn resolve_condrufs(&self, condrufs: CondRufs) -> Result<Vec<String>, AuditError> {
        // FIXME: impl resolve condrufs.
        unimplemented!()
    }

    fn check_rufs(&self, rustv: u32, rufs: &Vec<String>) -> bool {
        assert!(rustv < basic::RUSTC_VER_NUM as u32);
        if rufs
            .iter()
            .filter(|ruf| !basic::get_ruf_status(ruf, rustv).is_usable())
            .count()
            > 0
        {
            return false;
        }

        return true;
    }

    fn update_pkg(&self, name: &str, prev_ver: &str, new_ver: &str) -> Result<(), AuditError> {
        // FIXME: impl update_pkg.
        unimplemented!();
    }
}

#[test]
fn test_DepOpsVirt() {
    let depops = DepOpsVirt::new("caisin", "0.1.0", 0).unwrap();

    let tree = depops.get_deptree();
    println!("{:?}", tree);

    // FIXME: We need more tests on helper functions.
}
