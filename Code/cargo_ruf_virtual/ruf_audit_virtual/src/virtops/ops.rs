use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Mutex;

use cargo::core::registry::PackageRegistry;
use cargo::core::resolver::{CliFeatures, HasDevUnits};
use cargo::core::{PackageId, PackageIdSpec, PackageIdSpecQuery, Resolve, Shell, Workspace};
use cargo::util::cache_lock::CacheLockMode;
use cargo::util::interning::InternedString;
use cargo::{ops, GlobalContext};

use cargo_lock::dependency::Tree;
use cargo_lock::{Lockfile, Version};
use fxhash::{FxHashMap, FxHashSet};
use postgres::{Client, NoTls};
use regex::Regex;
use semver::VersionReq;

use crate::basic::{self, CondRuf, CondRufs};
use crate::core::AuditError;
use crate::core::DepOps;

lazy_static::lazy_static! {
    static ref RE_CONDS: Regex = Regex::new(r"^\s*feature\s*=\s*([\w-]+)\s*$").unwrap();
}

/*
    -- We have to strip empty cond('') to NULL --
    UPDATE version_feature SET conds = NULL WHERE conds = ''

    -- Currently we HAVE NOT created this table --
    CREATE VIEW version_ruf AS
    SELECT versions_with_name.id, versions_with_name.name, versions_with_name.num, versions_with_name.crate_id, version_feature.conds, version_feature.feature
    FROM versions_with_name
    JOIN version_feature
    ON versions_with_name.id = version_feature.id

    CREATE VIEW dependencies_with_name AS
    SELECT dependencies.*, crates.name AS crate_name
    FROM dependencies
    JOIN crates
    ON dependencies.crate_id = crates.id
*/

/// Colect needed info from our databases, we call it virtual impl.
/// Used for virtual pipeline analysis.
pub struct DepOpsVirt {
    /// For our database connection.
    conn: Mutex<Client>,

    /// For the target crates.
    name: String,
    ver: String,

    /// For the virt workspace.
    workspace_path: PathBuf,
    registry_path: PathBuf,
    toml_path: PathBuf,

    /// The local crates.
    locals: FxHashMap<String, FxHashMap<String, VersionReq>>,

    /// For the resolve result.
    resolve: Option<Resolve>,
    lockfile: Option<Lockfile>,
}

impl DepOpsVirt {
    pub fn new(name: &str, ver: &str, workspace: &str) -> Result<Self, AuditError> {
        // Prepare the db client.
        let client = Client::connect(
            "host=localhost dbname=crates user=postgres password=postgres",
            NoTls,
        )
        .unwrap();

        // Prepare local crates.
        let mut locals = FxHashMap::default();
        let mut virt_inner = FxHashMap::default();
        let virt_req = VersionReq::parse(&format!("={}", ver))
            .map_err(|e| AuditError::InnerError(e.to_string()))?;
        virt_inner.insert(name.to_string(), virt_req);
        locals.insert("virt".to_string(), virt_inner);

        let workspace_path = PathBuf::from(workspace);
        let registry_path = workspace_path.join("registry");
        let toml_path = workspace_path.join("Cargo.toml");

        let mut uninit = Self {
            conn: Mutex::new(client),

            name: name.to_string(),
            ver: ver.to_string(),

            workspace_path: workspace_path,
            registry_path: registry_path,
            toml_path: toml_path,

            locals: locals,

            resolve: None,
            lockfile: None,
        };

        uninit
            .first_resolve()
            .map_err(|e| AuditError::InnerError(e))?;

        Ok(uninit)
    }

    #[allow(unused)]
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

    fn get_cads_with_crate_name(&self, name: &str) -> Result<FxHashMap<Version, CondRufs>, String> {
        let rows = self
            .conn
            .lock()
            .unwrap()
            .query(
                "SELECT num, conds, feature FROM version_ruf WHERE name = $1",
                &[&name],
            )
            .map_err(|e| e.to_string())?;

        let mut dep_rufs = FxHashMap::default();
        for row in rows {
            let ver = row.get::<_, String>(0);
            let ver = Version::parse(&ver)
                .map_err(|e| format!("Version parse failure, invalid version: {} {}", ver, e))?;

            let entry = dep_rufs.entry(ver).or_insert_with(CondRufs::empty);

            let cond = row.get::<_, Option<String>>(1);
            let ruf = row.get::<_, String>(2);

            if ruf != "no_feature_used" {
                entry.push(CondRuf {
                    cond: cond,
                    feature: ruf,
                });
            }
        }

        Ok(dep_rufs)
    }

    fn get_reqs_with_version_id(
        &self,
        version_id: i32,
    ) -> Result<FxHashMap<String, VersionReq>, String> {
        let rows = self
            .conn
            .lock()
            .unwrap()
            .query(
                "SELECT crate_name, req FROM dependencies_with_name WHERE version_id = $1",
                &[&version_id],
            )
            .map_err(|e| e.to_string())?;

        let mut dep_reqs = FxHashMap::default();
        for row in rows {
            let name = row.get::<_, String>(0);
            let req = row.get::<_, String>(1);
            let req = VersionReq::parse(&req)
                .map_err(|e| format!("VersionReq parse failure, invalid req: {} {}", req, e))?;

            // FIXME: Shall we ignore the optional, target, etc on the dependencies ?
            let check_dup = dep_reqs.insert(name, req);
            assert!(check_dup.is_none());
        }

        Ok(dep_reqs)
    }

    /// For the inital resolve, called at [new] only once.
    fn first_resolve(&mut self) -> Result<(), String> {
        let mut features = Vec::new();

        // Create virtual environment.
        assert!(self.workspace_path.exists());

        // Get virtual toml file
        let file = self.format_virt_toml_file(&self.name, &self.ver, &features);
        File::create(&self.toml_path)
            .map_err(|e| e.to_string())?
            .write_all(file.as_bytes())
            .expect("Fatal, write virt.toml file failed");

        // 1. Pre-resolve: get all features first
        let config = GlobalContext::new(
            Shell::new(),
            self.workspace_path.clone(),
            self.registry_path.clone(),
        );
        let ws = Workspace::new(&self.toml_path, &config).map_err(|e| e.to_string())?;
        let mut registry = PackageRegistry::new(ws.gctx()).map_err(|e| e.to_string())?;
        let mut resolve = ops::resolve_with_previous(
            &mut registry,
            &ws,
            &CliFeatures::new_all(true),
            HasDevUnits::No,
            None,
            None,
            &[],
            true,
        )
        .map_err(|e| e.to_string())?;

        let pkg = resolve
            .query(&format!("{}@{}", &self.name, &self.ver))
            .map_err(|e| e.to_string())?;
        for feature in resolve.summary(pkg).features().keys() {
            features.push(feature.as_str());
        }

        // 2. Update resolve with features if found any.
        if !features.is_empty() {
            let file = self.format_virt_toml_file(&self.name, &self.ver, &features);
            File::create(&self.toml_path)
                .map_err(|e| e.to_string())?
                .write_all(file.as_bytes())
                .expect("Fatal, write virt.toml file failed");

            let config = GlobalContext::new(
                Shell::new(),
                self.workspace_path.clone(),
                self.registry_path.clone(),
            );
            let ws = Workspace::new(&self.toml_path, &config).map_err(|e| e.to_string())?;
            let mut registry = PackageRegistry::new(ws.gctx()).map_err(|e| e.to_string())?;

            resolve = ops::resolve_with_previous(
                &mut registry,
                &ws,
                &CliFeatures::new_all(true),
                HasDevUnits::No,
                None,
                None,
                &[],
                true,
            )
            .map_err(|e| e.to_string())?;
        }

        // And here the resolve is finally usable.
        let lockfile = ops::resolve_to_string(&ws, &mut resolve).map_err(|e| e.to_string())?;
        let lockfile = Lockfile::from_str(&lockfile).map_err(|e| e.to_string())?;

        // Updates the resolve and lockfile.
        self.resolve = Some(resolve);
        self.lockfile = Some(lockfile);

        Ok(())
    }

    fn update_resolve(&mut self, name: &str, prev_ver: &str, new_ver: &str) -> Result<(), String> {
        let previous_resolve = self.resolve.as_ref().unwrap();
        let name_ver = format!("{}@{}", name, prev_ver);

        let config = GlobalContext::new(
            Shell::new(),
            self.workspace_path.clone(),
            self.registry_path.clone(),
        );
        let ws = Workspace::new(&self.toml_path, &config).map_err(|e| e.to_string())?;

        let _lock = ws
            .gctx()
            .acquire_package_cache_lock(CacheLockMode::DownloadExclusive)
            .map_err(|e| e.to_string())?;

        let mut registry = PackageRegistry::new(ws.gctx()).map_err(|e| e.to_string())?;
        let mut to_avoid = HashSet::new();

        let mut sources = Vec::new();
        let dep = previous_resolve.query(&name_ver).unwrap();

        to_avoid.insert(dep);
        sources.push({
            assert!(dep.source_id().is_registry());
            dep.source_id()
                .with_precise_registry_version(dep.name(), dep.version().clone(), new_ver)
                .map_err(|e| e.to_string())?
        });

        if let Ok(unused_id) =
            PackageIdSpec::query_str(&name_ver, previous_resolve.unused_patches().iter().cloned())
        {
            to_avoid.insert(unused_id);
        }

        // Mirror `--workspace` and never avoid workspace members.
        // Filtering them out here so the above processes them normally
        // so their dependencies can be updated as requested
        to_avoid = to_avoid
            .into_iter()
            .filter(|id| {
                for package in ws.members() {
                    let member_id = package.package_id();
                    // Skip checking the `version` because `previous_resolve` might have a stale
                    // value.
                    // When dealing with workspace members, the other fields should be a
                    // sufficiently unique match.
                    if id.name() == member_id.name() && id.source_id() == member_id.source_id() {
                        return false;
                    }
                }
                true
            })
            .collect();

        registry.add_sources(sources).map_err(|e| e.to_string())?;

        // Here we place an artificial limitation that all non-registry sources
        // cannot be locked at more than one revision. This means that if a Git
        // repository provides more than one package, they must all be updated in
        // step when any of them are updated.
        //
        // OFFICAL TODO: this seems like a hokey reason to single out the registry as being
        // different.
        let to_avoid_sources: HashSet<_> = to_avoid
            .iter()
            .map(|p| p.source_id())
            .filter(|s| !s.is_registry())
            .collect();

        let keep =
            |p: &PackageId| !to_avoid_sources.contains(&p.source_id()) && !to_avoid.contains(p);

        let mut resolve = ops::resolve_with_previous(
            &mut registry,
            &ws,
            &CliFeatures::new_all(true),
            HasDevUnits::No,
            Some(&previous_resolve),
            Some(&keep),
            &[],
            true,
        )
        .map_err(|e| e.to_string())?;

        // ops::print_lockfile_changes(&ws, Some(previous_resolve), &resolve, &mut registry).unwrap();

        let lockfile = ops::resolve_to_string(&ws, &mut resolve).map_err(|e| e.to_string())?;
        let lockfile = Lockfile::from_str(&lockfile).map_err(|e| e.to_string())?;

        // Updates the resolve and lockfile.
        self.resolve = Some(resolve);
        self.lockfile = Some(lockfile);

        Ok(())
    }

    fn format_virt_toml_file(&self, name: &str, ver: &str, features: &Vec<&str>) -> String {
        let mut file = String::with_capacity(256);
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

            // If no ruf used, we just skip it.
            if !pkg_rufs.is_empty() {
                let name_ver = format!("{}@{}", pkg_id.name(), pkg_id.version());
                let check_dup = rufs.insert(name_ver, pkg_rufs);
                assert!(check_dup.is_none());
            }
        }

        Ok(rufs)
    }

    fn extract_rufs_from_one_pkg(
        &self,
        name: &str,
        ver: &str,
        pkg_feature: &[InternedString],
    ) -> Result<Vec<String>, String> {
        // println!(
        //     "[Debug] Extracting rufs from {}-{} with pf: {:?}",
        //     name, ver, pkg_feature
        // );
        let mut rufs = FxHashSet::default();
        let rows = self
            .conn
            .lock()
            .unwrap()
            .query(
                "SELECT conds, feature FROM version_ruf WHERE name = $1 AND num = $2 and feature != 'no_feature_used'",
                &[&name, &ver],
            )
            .map_err(|e| e.to_string())?;

        for row in rows {
            let cond = row.get::<usize, Option<String>>(0);
            let feature = row.get::<usize, String>(1);

            // Check the conditions and add the feature if enabled.
            if let Some(cond) = cond {
                assert!(!cond.is_empty());
                if let Some(caps) = RE_CONDS.captures(&cond) {
                    let cond_pf = caps.get(1).expect("Fatal, invalid regex capture").as_str();
                    // println!("[Debug] cond_pf: {}, ruf: {}", cond_pf, feature);
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

        self.get_cads_with_crate_name(name)
            .map_err(|e| AuditError::InnerError(e))
    }

    fn get_pkg_versionreq(
        &self,
        name: &str,
        ver: &str,
    ) -> Result<FxHashMap<String, VersionReq>, AuditError> {
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

    fn resolve_condrufs(
        &self,
        name: &str,
        ver: &str,
        condrufs: CondRufs,
    ) -> Result<Vec<String>, AuditError> {
        let mut rufs = FxHashSet::default();

        let resolve = self.resolve.as_ref().unwrap();
        let pkg_id = resolve
            .query(&format!("{}@{}", name, ver))
            .map_err(|e| AuditError::InnerError(e.to_string()))?;

        // FIXME: Do we get the correct pkg_features ?
        let pkg_features = resolve.features(pkg_id);

        for condruf in condrufs.inner() {
            if let Some(cond) = condruf.cond {
                assert!(!cond.is_empty());
                if let Some(caps) = RE_CONDS.captures(&cond) {
                    let cond_pf = caps.get(1).expect("Fatal, invalid regex capture").as_str();
                    if pkg_features.contains(&InternedString::new(cond_pf)) {
                        rufs.insert(condruf.feature);
                    }
                } // Or it's not `feature = "xxx"` condition, we assume it not enabled.
            } else {
                rufs.insert(condruf.feature);
            }
        }

        Ok(rufs.drain().collect())
    }

    fn check_rufs(&self, rustv: u32, rufs: &Vec<String>) -> bool {
        assert!(rustv < basic::RUSTC_VER_NUM as u32);

        let mut res = Vec::new();
    
        for ruf in rufs {
            if !basic::get_ruf_status(ruf, rustv).is_usable() {
                res.push(ruf.as_str());
            }
        }
        println!("[Debug] check_rufs: error rufs: {:?}",res);
    
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

    fn update_pkg(&mut self, name: &str, prev_ver: &str, new_ver: &str) -> Result<(), AuditError> {
        self.update_resolve(name, prev_ver, new_ver)
            .map_err(|e| AuditError::InnerError(e))
    }
}

#[test]
#[allow(non_snake_case)]
fn test_DepOpsVirt() {
    const WORKSPACE_PATH: &str = "/home/ubuntu/Workspaces/Cargo-Ecosystem-Monitor/Code/cargo_ruf_virtual/ruf_audit_virtual/virt_work";
    /*
        TO TEST: get_cads_with_crate_name, get_reqs_with_version_id, first_resolve, update_resolve, extract_rufs_from_current_resolve
    */
    let depops = DepOpsVirt::new("taxonomy", "0.3.1", WORKSPACE_PATH).unwrap();
    // let depops = DepOpsVirt::new("sdl2-sys", "0.0.34", WORKSPACE_PATH).unwrap();
    // let depops = DepOpsVirt::new("libc", "0.2.129", WORKSPACE_PATH).unwrap();
    // let depops = DepOpsVirt::new("rich-sdl2-rust", "0.11.2", WORKSPACE_PATH).unwrap();

    // let mut depops = DepOpsVirt::new("ahash", "0.7.0", WORKSPACE_PATH).unwrap();
    // depops.update_resolve("getrandom", "0.2.7", "0.2.6").unwrap();
    // depops.update_resolve("serde", "1.0.143", "1.0.140").unwrap();

    // let resolve = depops.resolve.as_ref().unwrap();
    // for pkg in resolve.iter() {
    //     println!(
    //         "{}-{}: {:?}",
    //         pkg.name(),
    //         pkg.version(),
    //         resolve.features(pkg)
    //     );

    //     println!(
    //         "{}-{}: {:?}",
    //         pkg.name(),
    //         pkg.version(),
    //         resolve.summary(pkg).features().keys()
    //     )
    // }

    let res = depops.extract_rufs_from_current_resolve().unwrap();
    for (pkg, rufs) in res {
        let usable = depops.check_rufs(63, &rufs);
        println!("{} rufs: {:#?} [usable: {}]", pkg, rufs, usable);
    }
}
