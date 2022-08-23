use cargo::core::registry::PackageRegistry;
use cargo::core::resolver::{CliFeatures, HasDevUnits, ResolveOpts};
use cargo::core::{PackageIdSpec, Shell, Summary, Workspace};
use cargo::ops;
use cargo::util::Config;

use std::collections::{HashMap, HashSet, VecDeque};
use std::env::{self, current_dir};
use std::fs::File;
use std::io::{self, Write};
use std::panic::{self, catch_unwind};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{Context, Result};
use crossbeam::channel::{self};
use log::{error, info, warn};
use postgres::{Client, NoTls};

struct VersionInfo {
    version_id: i32,
    crate_id: i32,
    name: String,
    num: String,
}

const THREAD_DATA_SIZE: i64 = 500;
const RERESOLVE_DATA_SIZE: i64 = 20;


/// Main Operation
/// Run dependency resolving in `workers` threads
/// Only process crates whose status = `status`
pub fn run_deps(workers: usize, status: &str) {
    let conn = Arc::new(Mutex::new(
        Client::connect(
            "host=localhost dbname=crates user=postgres password=postgres",
            NoTls,
        )
        .unwrap(),
    ));
    prebuild_db_table(Arc::clone(&conn));

    // Create channel
    let (tx, rx) = channel::bounded(workers);

    // Create threads
    let mut handles = vec![];
    for i in 0..workers {
        let conn = conn.clone();
        let rx = rx.clone();

        handles.push(thread::spawn(move || {
            while let Ok(versions) = rx.recv() {
                for v in versions {
                    let v = v as VersionInfo;
                    // Set panic hook and store into DB
                    let old_hook = panic::take_hook();
                    panic::set_hook({
                        let conn_copy = conn.clone();
                        Box::new(move |info| {
                            let err_message = format!("{:?}", info);
                            error!(
                                "Thread {}: Panic occurs, version - {}, info:{}",
                                i, v.version_id, err_message
                            );
                            store_resolve_error(
                                Arc::clone(&conn_copy),
                                v.version_id,
                                true,
                                err_message,
                            );
                        })
                    });
                    if catch_unwind(|| {
                        //  MAIN OPERATION: Dependency Resolution
                        if let Err(e) = resolve(i as u32, Arc::clone(&conn), &v) {
                            warn!(
                                "Resolve version {} fails, due to error: {}",
                                v.version_id, e
                            );
                            store_resolve_error(
                                Arc::clone(&conn),
                                v.version_id,
                                false,
                                format!("{:?}", e),
                            );
                        } else {
                            info!("Thread {}: Done version - {}", i, v.version_id);
                        }
                    })
                    .is_err()
                    {
                        error!("Thread {}: Panic occurs, version - {}", i, v.version_id);
                    }
                    panic::set_hook(old_hook); // Must after catch_unwind
                }
            }
        }));
    }

    // Get versions
    loop {
        let conn = Arc::clone(&conn);
        let query = format!(
            r#"SELECT id,crate_id,name,num FROM versions_with_name WHERE id in (
                SELECT version_id FROM deps_process_status WHERE status='{}' ORDER BY version_id asc LIMIT {}
                )"#,
            status, THREAD_DATA_SIZE
        );

        let rows = conn.lock().unwrap().query(&query, &[]).unwrap();

        if rows.is_empty() {
            break;
        } else {
            let query = format!(
                r#"UPDATE deps_process_status SET status='processing' WHERE version_id IN (
                    SELECT version_id FROM process_status WHERE status='{}' ORDER BY version_id asc LIMIT {}
                )"#,
                status, THREAD_DATA_SIZE
            );

            conn.lock().unwrap().query(&query, &[]).unwrap();

            let versions: Vec<VersionInfo> = rows
                .iter()
                .map(|row| VersionInfo {
                    version_id: row.get(0),
                    crate_id: row.get(1),
                    name: row.get(2),
                    num: row.get(3),
                })
                .collect();

            tx.send(versions).unwrap();
        }
    }

    /*
    // Re-resolve: Resolve all unresolved crates.
    warn!("Re-resolve start");
    // Build cache table of unresolved crates
    conn.lock()
        .unwrap()
        .query(
            &format!("DROP TABLE IF EXISTS tmp_cached_ver_feature "),
            &[],
        )
        .unwrap();
    conn.lock()
        .unwrap()
        .query(
            &format!(
                "CREATE TABLE tmp_cached_ver_feature AS (
        WITH ver_feature AS
        (SELECT id as version_id, crate_id, num FROM versions WHERE id in
            (WITH ver_dep AS
                (SELECT DISTINCT version_id as ver FROM dependencies WHERE kind != 2)
            SELECT ver FROM ver_dep
            WHERE ver NOT IN (SELECT id FROM versions WHERE yanked = true)
            AND ver NOT IN (SELECT DISTINCT ver FROM dep_errors)
            AND ver NOT IN (SELECT DISTINCT version_from FROM dep_version))
        )
        SELECT version_id, crate_id, name, num FROM crates INNER JOIN ver_feature ON crate_id=id
        )
        "
            ),
            &[],
        )
        .unwrap();
    // Start sending data
    offset = 0;
    loop {
        let conn = Arc::clone(&conn);
        let query = format!(
            "SELECT * FROM tmp_cached_ver_feature LIMIT {} OFFSET {}",
            RERESOLVE_DATA_SIZE, offset
        );
        let rows = conn.lock().unwrap().query(&query, &[]).unwrap();

        if rows.is_empty() {
            break;
        } else {
            let v: Vec<VersionInfo> = rows
                .iter()
                .map(|info| VersionInfo {
                    version_id: info.get(0),
                    crate_id: info.get(1),
                    name: info.get(2),
                    num: info.get(3),
                })
                .collect();
            tx.send(v).expect("Send task error!");
        }
        offset += RERESOLVE_DATA_SIZE;
        warn!("Resolve OFFSET = {}", offset);
    }
    */

    std::mem::drop(tx);
    for handle in handles {
        // Unsolved problem
        if handle.join().is_err() {
            error!("!!!Thread Crash!!!")
        }
    }

    info!(r#"\\\ !Resolving Done! ///"#);
}


/// Wrapper of [`resolve_store_deps_of_version`]
fn resolve(thread_id: u32, conn: Arc<Mutex<Client>>, version_info: &VersionInfo) -> Result<()> {
    let version_id = version_info.version_id;
    let res = resolve_store_deps_of_version(thread_id, Arc::clone(&conn), version_info);

    let status = if res.is_ok() { "done" } else { "fail" };

    conn.lock()
        .unwrap()
        .query(
            &format!(
                "UPDATE process_status SET status = '{}' WHERE version_id = '{}';",
                status, version_id
            ),
            &[],
        )
        .expect("Update process status fails");

    res
}

/// Resolve version's dependencies and store them into db.
fn resolve_store_deps_of_version(
    thread_id: u32,
    conn: Arc<Mutex<Client>>,
    version_info: &VersionInfo,
) -> Result<()> {
    let version_id = version_info.version_id;
    let name = &version_info.name;
    let num = &version_info.num;
    let mut features = Vec::new();

    // Create virtual env by creating toml file
    // Fill toml contents
    let current_path = current_dir()?;
    let dep_filename = format!("dep{}.toml", thread_id);
    let current_toml_path = format!("{}/{}", current_path.display(), dep_filename);

    // Create toml file
    let file = format_virt_toml_file(&name, &num, &features);
    File::create(&current_toml_path)?
        .write_all(file.as_bytes())
        .expect("Write failed");

    // 1. Pre Resolve: To find all features of given crate
    // Create virtual env by setting correct workspace
    let config = Config::new(
        Shell::new(),
        env::current_dir()?,
        format!("{}/job{}", current_path.to_str().unwrap(), thread_id).into(),
    );
    let ws = Workspace::new(&Path::new(&current_toml_path), &config)?;
    let mut registry = PackageRegistry::new(ws.config())?;
    let resolve = ops::resolve_with_previous(
        &mut registry,
        &ws,
        &CliFeatures::new_all(true),
        HasDevUnits::No,
        None,
        None,
        &[],
        true,
    )?;

    // Find all `features` including user-defined and optional dependency
    if let Ok(res) = resolve.query(&format!("{}:{}", name, num)) {
        for feature in resolve.summary(res).features().keys() {
            features.push(feature.as_str());
        }
    } else {
        warn!("Resolve version {} fails to find any features.", version_id);
    }
    // println!("All Features: {:?}", features);

    // 2. Double resolve: This time resolve with features
    // The resolve result is the final one.
    let file = format_virt_toml_file(&name, &num, &features);
    // println!("file: {}", file);
    File::create(&current_toml_path)?
        .write_all(file.as_bytes())
        .expect("Write failed");
    let config = Config::new(
        Shell::new(),
        env::current_dir()?,
        format!("{}/job{}", current_path.to_str().unwrap(), thread_id).into(),
    );
    let ws = Workspace::new(&Path::new(&current_toml_path), &config).unwrap();
    let mut registry = PackageRegistry::new(ws.config()).unwrap();
    let resolve = ops::resolve_with_previous(
        &mut registry,
        &ws,
        &CliFeatures::new_all(true),
        // &features,
        HasDevUnits::No,
        None,
        None,
        &[],
        // &[PackageIdSpec::parse("dep").unwrap()],
        true,
    )
    .unwrap();

    // 3. Start Formatting Resolve and store into DB
    let mut map = HashMap::new();
    let mut set = HashSet::new();
    for pkg in resolve.iter() {
        map.insert(
            (pkg.name().to_string(), pkg.version().to_string()),
            get_version_by_name_version(
                Arc::clone(&conn),
                &pkg.name().to_string(),
                &pkg.version().to_string(),
            )?,
        );
    }
    // println!("{:#?}", map);

    // Resolve the dep tree.
    let root = resolve.query(&name)?;
    let mut v = VecDeque::new();
    let mut level = 1;
    v.extend([Some(root), None]);

    while let Some(next) = v.pop_front() {
        if let Some(pkg) = next {
            for (pkg, _) in resolve.deps(pkg) {
                set.insert((
                    map[&(pkg.name().to_string(), pkg.version().to_string())],
                    level,
                ));
                v.push_back(Some(pkg));
            }
        } else {
            level += 1;
            if !v.is_empty() {
                v.push_back(None)
            }
        }
    }

    // Store dep info into DB.
    if !set.is_empty() {
        let mut query = String::from("INSERT INTO dep_version VALUES");
        for (version_to, level) in set {
            query.push_str(&format!("({}, {}, {}),", version_id, version_to, level,));
        }
        query.pop();
        query.push(';');
        conn.lock().unwrap().query(&query, &[]).unwrap_or_default();
    }

    Ok(())
}

fn prebuild_db_table(conn: Arc<Mutex<Client>>){
    conn.lock()
    .unwrap()
    .query(
        r#"CREATE TABLE IF NOT EXISTS dep_version(
                    version_from INT,
                    version_to INT,
                    dep_level INT,
                    UNIQUE(version_from, version_to, dep_level))"#,
        &[],
    )
    .unwrap_or_default();
    conn.lock()
        .unwrap()
        .query(
            r#"CREATE TABLE IF NOT EXISTS public.dep_errors
            (
                ver integer,
                is_panic boolean,
                error text COLLATE pg_catalog."default",
                CONSTRAINT dep_errors_ver_is_panic_error_key UNIQUE (ver, is_panic, error)
            )"#,
            &[],
        )
        .unwrap_or_default();
    conn.lock().unwrap().query(r#"CREATE VIEW versions_with_name as (
        SELECT versions.*, crates.name FROM versions INNER JOIN crates ON versions.crate_id = crates.id
        )"#, &[]).unwrap_or_default();
    
}

/// Get a name by crate id.
///
/// # Example
/// ```
/// let name = get_name_by_crate_id(conn, 321)?;
/// assert_eq!(&name, "ecs")
/// ```
fn get_name_by_crate_id(conn: Arc<Mutex<Client>>, crate_id: i32) -> Result<String> {
    let query = format!("SELECT name FROM crates WHERE id = {} LIMIT 1", crate_id);
    let row = conn.lock().unwrap().query(&query, &[]).unwrap();
    Ok(row
        .first()
        .with_context(|| format!("Get name by crate id fails, crate id: {}", crate_id))?
        .get(0))
}

/// Get a name by version id.
///
/// # Example
/// ```
/// let name = get_name_by_version_id(conn, 6034)?;
/// assert_eq!(&name, "ecs")
/// ```
fn get_name_by_version_id(conn: Arc<Mutex<Client>>, version_id: i32) -> Result<String> {
    let query = format!(
        "SELECT crate_id FROM versions WHERE id = {} LIMIT 1",
        version_id
    );
    let row = conn.lock().unwrap().query(&query, &[]).unwrap();
    let crate_id = row
        .first()
        .with_context(|| format!("Get name by version id fails, version id: {}", version_id))?
        .get(0);
    get_name_by_crate_id(conn, crate_id)
}

fn get_features_by_version_id(conn: Arc<Mutex<Client>>, version_id: i32) -> Result<Vec<String>> {
    let mut features: HashSet<String> = HashSet::new();

    // Get User-defined Features
    let query = format!(
        "SELECT features FROM versions WHERE id = {} LIMIT 1",
        version_id
    );
    let row = conn.lock().unwrap().query(&query, &[]).unwrap();
    let results: serde_json::Value = row
        .first()
        .with_context(|| {
            format!(
                "Get version string by version id fails, version id: {}",
                version_id
            )
        })?
        .get(0);
    if let Some(vec_results) = results.as_object() {
        for (feature, _) in vec_results {
            features.insert(feature.clone());
        }
    }

    // Get optional dependency feature
    // let query = format!("SELECT name FROM dependencies INNER JOIN crates ON crates.id=crate_id
    //                     WHERE dependencies.version_id = {} AND optional = true"
    //                     , version_id);
    // let rows = conn.lock().unwrap().query(&query, &[]).unwrap();
    // for optional_dep_name in rows{
    //     features.insert(optional_dep_name.get(0));
    // }
    Ok(features.into_iter().collect::<Vec<_>>())
}

fn get_features_from_serde_json(
    conn: Arc<Mutex<Client>>,
    version_id: i32,
    results: serde_json::Value,
) -> Vec<String> {
    let mut features: HashSet<String> = HashSet::new();

    // Get User-defined Features
    if let Some(vec_results) = results.as_object() {
        for (feature, _) in vec_results {
            features.insert(feature.clone());
        }
    }

    // Get optional dependency feature
    let query = format!(
        "SELECT name FROM dependencies INNER JOIN crates ON crates.id=crate_id 
                        WHERE dependencies.version_id = {} AND optional = true",
        version_id
    );
    let rows = conn.lock().unwrap().query(&query, &[]).unwrap();
    for optional_dep_name in rows {
        features.insert(optional_dep_name.get(0));
    }
    features.into_iter().collect::<Vec<_>>()
}

/// Get give version's version string.
fn get_version_str_by_version_id(conn: Arc<Mutex<Client>>, version_id: i32) -> Result<String> {
    let query = format!("SELECT num FROM versions WHERE id = {} LIMIT 1", version_id);
    let row = conn.lock().unwrap().query(&query, &[]).unwrap();
    Ok(row
        .first()
        .with_context(|| {
            format!(
                "Get version string by version id fails, version id: {}",
                version_id
            )
        })?
        .get(0))
}

/// Get crate id by name.
///
/// # Example
/// ```
/// let crate_id = get_crate_id_by_name(conn, http);
/// assert_eq!(crate_id, 184);
/// ```
fn get_crate_id_by_name(conn: Arc<Mutex<Client>>, name: &str) -> Result<i32> {
    let query = format!("SELECT id FROM crates WHERE name = '{}' LIMIT 1", name);
    let row = conn.lock().unwrap().query(&query, &[]).unwrap();
    Ok(row
        .first()
        .with_context(|| format!("Get crate id by name fails, name: {}", name))?
        .get(0))
}

/// Get version by name and version string
///
/// # Example
/// ```
/// let version_id = get_version_by_name_version(conn, "http", "0.2.4");
/// assert_eq!(version_id, 362968);
/// ```
fn get_version_by_name_version(conn: Arc<Mutex<Client>>, name: &str, version: &str) -> Result<i32> {
    let query = format! {
        "SELECT id FROM versions WHERE num = '{}' AND crate_id = '{}' LIMIT 1",
        version,
        get_crate_id_by_name(Arc::clone(&conn), name)?
    };
    let row = conn.lock().unwrap().query(&query, &[]).unwrap();
    Ok(row
        .first()
        .with_context(|| {
            format!(
                "Get version id by name & version fails, name:{} version: {}",
                name, version
            )
        })?
        .get(0))
}

/// Get version by name and version string
///
/// # Example
/// ```
/// store_resolve_error(conn, 362968, false, String::new("Error"));
/// ```
fn store_resolve_error(conn: Arc<Mutex<Client>>, version: i32, is_panic: bool, message: String) {
    let message = message.replace("'", "''");
    let query = format! {
        "INSERT INTO dep_errors(ver, is_panic, error) VALUES ({}, {:?}, '{}');",
        version, is_panic, message
    };
    conn.lock().unwrap().query(&query, &[]).unwrap_or_default();
}




fn format_virt_toml_file(name: &String, version_num: &String, features: &Vec<&str>) -> String {
    let mut file = String::from(
        r#"[package]
name = "dep"
version = "0.1.0"
[dependencies]"#,
    );
    file.push('\n');

    // Add all features
    file.push_str(&format!(
        "{} = {}version = \"={}\", features = [",
        name, "{", version_num
    ));
    for feature in features {
        file.push_str(&format!("\"{}\",", feature));
    }
    file.push_str("]}");
    file
}

#[test]
fn resolve_test() -> io::Result<()> {
    let conn = Arc::new(Mutex::new(
        Client::connect(
            "host=localhost dbname=crates user=postgres password=postgres",
            NoTls,
        )
        .unwrap(),
    ));
    let thread_id = 999;
    let name = String::from("openmls");
    let num = String::from("0.4.1");
    let mut features = Vec::new();
    // let features:Vec<String> = Vec::new();

    // Create virtual env by creating toml file
    // Fill toml contents
    let current_path = current_dir()?;
    let dep_filename = format!("dep{}.toml", thread_id);
    let current_toml_path = format!("{}/{}", current_path.display(), dep_filename);

    // Create virtual env by setting correct workspace
    let file = format_virt_toml_file(&name, &num, &features);
    println!("file: {}", file);
    File::create(&current_toml_path)?
        .write_all(file.as_bytes())
        .expect("Write failed");

    // Pre Resolve: To find all features of given crate
    let config = Config::new(
        Shell::new(),
        env::current_dir()?,
        format!("{}/job{}", current_path.to_str().unwrap(), thread_id).into(),
    );
    let mut ws = Workspace::new(&Path::new(&current_toml_path), &config).unwrap();
    let mut registry = PackageRegistry::new(ws.config()).unwrap();
    // println!("Workspace: {:?}", ws);
    ws.set_require_optional_deps(false);
    let resolve = ops::resolve_with_previous(
        &mut registry,
        &ws,
        &CliFeatures::new_all(true),
        // &features,
        HasDevUnits::No,
        None,
        None,
        &[],
        // &[PackageIdSpec::parse("dep").unwrap()],
        true,
    )
    .unwrap();
    // println!("{:#?}", resolve);

    // Find all `features` including user-defined and optional dependency
    if let Ok(res) = resolve.query(&format!("{}:{}", name, num)) {
        for feature in resolve.summary(res).features().keys() {
            features.push(feature.as_str());
        }
    } else {
        println!("NO RES");
    }
    println!("All Features: {:?}", features);

    // Double resolve: This time resolve with features
    let file = format_virt_toml_file(&name, &num, &features);
    println!("file: {}", file);
    File::create(&current_toml_path)?
        .write_all(file.as_bytes())
        .expect("Write failed");
    let config = Config::new(
        Shell::new(),
        env::current_dir()?,
        format!("{}/job{}", current_path.to_str().unwrap(), thread_id).into(),
    );
    let ws = Workspace::new(&Path::new(&current_toml_path), &config).unwrap();
    let mut registry = PackageRegistry::new(ws.config()).unwrap();
    let resolve = ops::resolve_with_previous(
        &mut registry,
        &ws,
        &CliFeatures::new_all(true),
        // &features,
        HasDevUnits::No,
        None,
        None,
        &[],
        // &[PackageIdSpec::parse("dep").unwrap()],
        true,
    )
    .unwrap();
    println!("{:#?}", resolve);

    Ok(())

    // // User defined feature
    // let user_features = resolve.features(res);
    // for user_feature in user_features {
    //     features.insert(user_feature.as_str());
    // }
    // // Optinal Dependency (Renaming ones)
    // let iter = resolve.deps(res);
    // for (_, deps) in iter{
    //     for dep in deps {
    //         if dep.is_optional() {
    //             features.insert(dep.name_in_toml().as_str());
    //         }
    //     }
    // }

    // let mut map = HashMap::new();
    // let mut set = HashSet::new();

    // for pkg in resolve.iter() {
    //     map.insert(
    //         (pkg.name().to_string(), pkg.version().to_string()),
    //         get_version_by_name_version(
    //             Arc::clone(&conn),
    //             &pkg.name().to_string(),
    //             &pkg.version().to_string(),
    //         )?,
    //     );
    // }

    // // println!("{:#?}", map);

    // // Resolve the dep tree.
    // let root = resolve.query(&name)?;
    // let mut v = VecDeque::new();
    // let mut level = 1;
    // v.extend([Some(root), None]);

    // while let Some(next) = v.pop_front() {
    //     if let Some(pkg) = next {
    //         for (pkg, _) in resolve.deps(pkg) {
    //             set.insert((
    //                 map[&(pkg.name().to_string(), pkg.version().to_string())],
    //                 level,
    //             ));
    //             v.push_back(Some(pkg));
    //         }
    //     } else {
    //         level += 1;
    //         if !v.is_empty() {
    //             v.push_back(None)
    //         }
    //     }
    // }

    // // Store dep info into DB.
    // if ! set.is_empty(){
    //     let mut query = String::from("INSERT INTO dep_version VALUES");
    //     for (version_to, level) in set {
    //         query.push_str(&format!(
    //             "({}, {}, {}),",
    //             version_id, version_to, level,
    //         ));
    //     }
    //     query.pop();
    //     query.push(';');
    //     conn.lock().unwrap().query(&query, &[]).unwrap_or_default();
    // }
}
