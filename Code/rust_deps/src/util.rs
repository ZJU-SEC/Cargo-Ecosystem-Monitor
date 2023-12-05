use cargo::core::registry::PackageRegistry;
use cargo::core::resolver::{CliFeatures, HasDevUnits};
use cargo::core::{Workspace, Shell};
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

// use crate::graph::{self, Node};

pub struct VersionInfo {
    pub version_id: i32,
    pub crate_id: i32,
    pub name: String,
    pub num: String,
}

pub const THREAD_DATA_SIZE: i64 = 50;
pub const RERESOLVE_DATA_SIZE: i64 = 20;

// Suffix of DB, used for test and other purposes.
// It is empty by default. If it is not, it is not used for general purposes.
pub const DB_SUFFIX: &str = "";


/// Main Operation
/// Run dependency resolving in `workers` threads
/// Only process crates whose status = `status`
pub fn run_deps(workers: usize, status: &str) {
    if status == "processing"{
        panic!("If you specify undone, it will automatically 
        process crates whose status is 'processing'")
    }
    if status != "undone" &&
       status != "fail"{
        panic!("The status can only be undone/fail")
    }

    let conn = Arc::new(Mutex::new(
        Client::connect(
            "host=localhost dbname=crates user=postgres password=postgres",
            NoTls,
        )
        .unwrap(),
    ));
    println!("DB Prebuild");
    prebuild_db_table(Arc::clone(&conn));
    // let ver_name_table = get_ver_name_table(Arc::clone(&conn));
    let ver_name_table = Arc::new(get_ver_name_table(Arc::clone(&conn)));

    println!("Creating Channel");
    // Create channel
    let (tx, rx) = channel::bounded(workers);

    // Create threads
    let mut handles = vec![];
    for i in 0..workers {
        let conn = conn.clone();
        let rx = rx.clone();
        let ver_name_table = ver_name_table.clone();

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
                        if let Err(e) = resolve(i as u32, Arc::clone(&conn), &v, Arc::clone(&ver_name_table)) {
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
                        store_resolve_error(
                            Arc::clone(&conn),
                            v.version_id,
                            true,
                            "".to_string(),
                        );
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
                SELECT version_id FROM deps_process_status{DB_SUFFIX} WHERE status='{}' ORDER BY version_id asc LIMIT {}
                )"#,
                status, THREAD_DATA_SIZE
        );

        let rows = conn.lock().unwrap().query(&query, &[]).unwrap();

        if rows.is_empty() {
            break;
        } else {
            let query = format!(
                r#"UPDATE deps_process_status{DB_SUFFIX} SET status='processing' WHERE version_id IN (
                    SELECT version_id FROM deps_process_status{DB_SUFFIX} WHERE status='{}' ORDER BY version_id asc LIMIT {}
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
fn resolve(
    thread_id: u32, 
    conn: Arc<Mutex<Client>>, 
    version_info: &VersionInfo,
    ver_name_table: Arc<HashMap<(String, String), i32>>,
) -> Result<()> {
    let version_id = version_info.version_id;
    let res = resolve_store_deps_of_version(thread_id, Arc::clone(&conn), version_info, ver_name_table);

    let status = if res.is_ok() { "done" } else { "fail" };

    update_process_status(Arc::clone(&conn), version_id, status);

    res
}

pub fn update_process_status(conn: Arc<Mutex<Client>>, version_id: i32, status: &str){
    // warn!("update status");
    conn.lock()
        .unwrap()
        .query(
            &format!(
                "UPDATE deps_process_status{DB_SUFFIX} SET status = '{}' WHERE version_id = '{}';",
                status, version_id
            ),
            &[],
        )
        .expect("Update process status fails");
}

/// Resolve version's dependencies and store them into db.
fn resolve_store_deps_of_version(
    thread_id: u32,
    conn: Arc<Mutex<Client>>,
    version_info: &VersionInfo,
    ver_name_table: Arc<HashMap<(String, String), i32>>,
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
            get_version_by_name_version_test(
                Arc::clone(&ver_name_table),
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
        let mut query = format!("INSERT INTO dep_version{DB_SUFFIX} VALUES");
        for (version_to, level) in set {
            query.push_str(&format!("({}, {}, {}),", version_id, version_to, level,));
        }
        query.pop();
        query.push(';');
        conn.lock().unwrap().query(&query, &[]).unwrap_or_default();
    }

    Ok(())
}

pub fn prebuild_db_table(conn: Arc<Mutex<Client>>){
    conn.lock()
    .unwrap()
    .query(
        &format!(r#"CREATE TABLE IF NOT EXISTS dep_version{DB_SUFFIX}(
                    version_from INT,
                    version_to INT,
                    dep_level INT,
                    UNIQUE(version_from, version_to, dep_level))"#),
        &[],
    )
    .unwrap_or_default();
    conn.lock()
        .unwrap()
        .query(
            &format!(r#"CREATE TABLE IF NOT EXISTS public.dep_errors{DB_SUFFIX}
            (
                ver integer,
                is_panic boolean,
                error text COLLATE pg_catalog."default",
                CONSTRAINT dep_errors_ver_is_panic_error_key UNIQUE (ver, is_panic, error)
            )"#),
            &[],
        )
        .unwrap_or_default();
    conn.lock().unwrap().query(&format!(r#"CREATE VIEW versions_with_name as (
        SELECT versions.*, crates.name FROM versions INNER JOIN crates ON versions.crate_id = crates.id
        )"#), &[]).unwrap_or_default();
    // Crate resolution process
    conn.lock()
        .unwrap()
        .query(
            &format!(r#"CREATE TABLE IF NOT EXISTS public.deps_process_status{DB_SUFFIX}
            (
                version_id INT,
                status VARCHAR
            )"#),
            &[],
        )
        .unwrap();
    // Check if table is empty
    if conn.lock().unwrap().query(
        &format!("SELECT * FROM deps_process_status{DB_SUFFIX} LIMIT 1"),
            &[],
        ).unwrap().first().is_none()
    {
        conn.lock().unwrap()
            .query(&format!("
                WITH ver_dep AS
                        (SELECT DISTINCT version_id as ver FROM dependencies WHERE kind != 2)
                INSERT INTO public.deps_process_status{DB_SUFFIX} 
                    SELECT ver, 'undone' FROM ver_dep
                    WHERE ver NOT IN (SELECT id FROM versions WHERE yanked = true)
                    AND ver NOT IN (SELECT DISTINCT ver FROM dep_errors{DB_SUFFIX})
                    AND ver NOT IN (SELECT DISTINCT version_from FROM dep_version{DB_SUFFIX})"),
                &[],
            ).unwrap();
    }
    else{
        let query = format!(
            r#"UPDATE deps_process_status{DB_SUFFIX} SET status='undone' WHERE version_id IN (
                SELECT version_id FROM deps_process_status{DB_SUFFIX} WHERE status='processing'
            )"#
        );
        conn.lock().unwrap().query(&query, &[]).unwrap();
    }
}

pub fn get_ver_name_table(conn: Arc<Mutex<Client>>) -> HashMap<(String, String), i32>{
    let rows = conn.lock()
    .unwrap()
    .query(
        &format!(r#"SELECT id, crate_id, num ,name FROM versions_with_name"#),
        &[],
    ).unwrap();
    // <(name, num), version_id>
    let mut ver_name_table:HashMap<(String, String), i32> = HashMap::new();
    for ver in rows{
        let name:String = ver.get(3);
        let num:String = ver.get(2);
        let version_id:i32 = ver.get(0);
        ver_name_table.entry((name, num)).or_insert(version_id);
    }
    ver_name_table

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


// Although it is marked as `test`, it performs better than the original code. We will do code refactor later.
pub fn get_version_by_name_version_test(table: Arc<HashMap<(String, String), i32>>, name: &str, version: &str) -> Result<i32> {
    Ok(
        table.get(&(name.to_string(), version.to_string()))
            .context("Can't get version_id")?
            .clone())
}

/// Get version by name and version string
///
/// # Example
/// ```
/// store_resolve_error(conn, 362968, false, String::new("Error"));
/// ```
pub fn store_resolve_error(conn: Arc<Mutex<Client>>, version: i32, is_panic: bool, message: String) {
    let message = message.replace("'", "''");
    let query = format! {
        "INSERT INTO dep_errors{DB_SUFFIX}(ver, is_panic, error) VALUES ({}, {:?}, '{}');",
        version, is_panic, message
    };
    conn.lock().unwrap().query(&query, &[]).unwrap_or_default();
    update_process_status(Arc::clone(&conn), version, "fail");
}




fn format_virt_toml_file(name: &String, version_num: &String, features: &Vec<&str>) -> String {
    let mut file = String::from(
        r#"[package]
name = "dep"
version = "0.1.0"
edition = "2021"
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




// Write `dependencies` to `file` in csv format, sorted.
pub fn write_dependency_file_sorted(path_string: String, dependencies: &HashMap<String, HashSet<String>>){
    let mut content:Vec<String> = Vec::new();
    let path = Path::new(path_string.as_str());
    let display = path.display();   
    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't create {}: {}", display, why),
        Ok(file) => file,
    };
    for(crate_name, versions) in dependencies {
        for version in versions {
            let dep_name = format!("{},", crate_name);
            let dep_ver = format!("{}", version);
            let line = dep_name+ &dep_ver + "\n";
            content.push(line);
        }
    }
    content.sort();
    for line in content {
        if let Err(why) = file.write_all(line.as_bytes()) {
            panic!("couldn't write to {}: {}", path.display(), why);
        }
    }
}





#[test]
fn resolve_test() -> io::Result<()> {
    let conn = Arc::new(Mutex::new(
        Client::connect(
            "host=localhost port=5432 dbname=crates user=postgres password=postgres",
            NoTls,
        )
        .unwrap(),
    ));
    let ver_name_table = Arc::new(get_ver_name_table(Arc::clone(&conn)));
    
    println!("{:#?}", get_version_by_name_version_test(Arc::clone(&ver_name_table), "tin-summer", "1.21.3"));
    // return Ok(());
    let thread_id = 999;
    let name = String::from("caisin");
    let num = String::from("0.1.57");
    let mut features = Vec::new();
    // let features:Vec<String> = Vec::new();

    // Create virtual env by creating toml file
    // Fill toml contents
    let current_path = current_dir()?;
    let dep_filename = format!("dep{}.toml", thread_id);
    let current_toml_path = format!("{}/{}", current_path.display(), dep_filename);
    // let current_toml_path = format!("/home/loancold/projects/Cargo-Ecosystem-Monitor/Code/demo/feature_level_dependency/Cargo.toml");

    // Create virtual env by setting correct workspace
    let file = format_virt_toml_file(&name, &num, &features);
    // println!("file: {}", file);
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
    ws.set_require_optional_deps(true);
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
    // println!("{:#?}", ws);
    // return Ok(());
    // println!("{:#?}", resolve);

    // Find all `features` including user-defined and optional dependency
    if let Ok(res) = resolve.query(&format!("{}:{}", name, num)) {
        for feature in resolve.summary(res).features().keys() {
            features.push(feature.as_str());
        }
    } else {
        println!("NO RES");
    }
    // println!("All Features: {:?}", features);

    // Double resolve: This time resolve with features
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

    // // Count Deps
    // let mut count_dep = 0;
    // for pkg in resolve.iter() {
    //     let dep_num = resolve.deps(pkg).count();
    //     let dep_name = pkg.name().to_string();
    //     println!("{dep_name} + {dep_num}");
    //     count_dep += dep_num;
    // }
    // count_dep -= 1; // Remove pkg 'dep' (our virtual pkg).
    // println!("{:#?}", resolve);
    // println!("Dep count: {}", count_dep);

    // {
    //     let R = "clang-sys";
    //     let num = "1.3.3";
    //     let v = format!("{}:{}", name, num);
    //     let query_feature = "clang_3_9";
    //     let nightly_feature = "THis one";
    //     let mut tmp_features:Vec<&str> = Vec::new();
    //     if let Ok(res) = resolve.query(&v) {
    //         for feature in resolve.features(res) {
    //             if feature == query_feature{
    //                 tmp_features.push(nightly_feature);
    //             }W
    //         }
    //     }
    //     println!("{:#?}", tmp_features);
    // }

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



// Deprecated functions (for reference only)


// fn fix_resolve(path: &str, name: &str) -> Result<()> {
//     let config = Config::default()?;

//     let ws = Workspace::new(&Path::new(&format!("{path}/Cargo.toml")), &config)?;
//     let requested_targets = Vec::new();
//     let requested_kinds = CompileKind::from_requested_targets(ws.config(), &requested_targets)?;
//     let target_data = RustcTargetData::new(&ws, &requested_kinds)?;

//     let specs = PackageIdSpec::query_str(name, ws.members().map(|pkg| pkg.package_id()))?;
//     let specs = [PackageIdSpec::from_package_id(specs)];

//     let ws_resolve = ops::resolve_ws_with_opts(
//         &ws,
//         &target_data,
//         &requested_kinds,
//         &CliFeatures::new_all(false),
//         &specs,
//         HasDevUnits::Yes,
//         ForceAllTargets::No,
//     )?;

//     let package_map: HashMap<PackageId, &Package> = ws_resolve
//         .pkg_set
//         .packages()
//         .map(|pkg| (pkg.package_id(), pkg))
//         .collect();

//     // Default tree options
//     let cli_features = CliFeatures::new_all(false);
//     let packages = Packages::Default;
//     let target = Target::Host;
//     let mut edge_kinds = HashSet::new();
//     edge_kinds.insert(EdgeKind::Dep(DepKind::Normal));
//     edge_kinds.insert(EdgeKind::Dep(DepKind::Build));
//     edge_kinds.insert(EdgeKind::Dep(DepKind::Development));
//     let invert = vec![];
//     let pkgs_to_prune = vec![];
//     let prefix = Prefix::Indent;
//     let no_dedupe = false;
//     let duplicates = false;
//     let charset = Charset::Utf8;
//     let format = "{p}".to_string();
//     let graph_features = false;
//     let max_display_depth = u32::MAX;
//     let no_proc_macro = false;

//     let opts = TreeOptions {
//         cli_features,
//         packages,
//         target,
//         edge_kinds,
//         invert,
//         pkgs_to_prune,
//         prefix,
//         no_dedupe,
//         duplicates,
//         charset,
//         format,
//         graph_features,
//         max_display_depth,
//         no_proc_macro,
//     };


//     let mut g = graph::build(
//         &ws,
//         &ws_resolve.targeted_resolve,
//         &ws_resolve.resolved_features,
//         &specs,
//         &CliFeatures::new_all(false),
//         &target_data,
//         &requested_kinds,
//         package_map,
//         &opts,
//     )?;

//     println!("{:?}", g.nodes);

//     Ok(())
// }

// #[test]
// fn resolve_test_fixed() -> Result<()> {
//     let conn = Arc::new(Mutex::new(
//         Client::connect(
//             "host=localhost port=5434 dbname=crates user=postgres password=postgres",
//             NoTls,
//         )
//         .unwrap(),
//     ));
//     let ver_name_table = Arc::new(get_ver_name_table(Arc::clone(&conn)));
    
//     let thread_id = 999;
//     let name = String::from("caisin");
//     let num = String::from("0.1.57");
//     let mut features = Vec::new();

//     // Create virtual env by creating toml file
//     // Fill toml contents
//     let current_path = current_dir()?;
//     let dep_filename = format!("dep{}.toml", thread_id);
//     let current_toml_path = format!("{}/{}", current_path.display(), dep_filename);

//     // Create virtual env by setting correct workspace
//     let file = format_virt_toml_file(&name, &num, &features);
//     // println!("file: {}", file);
//     File::create(&current_toml_path)?
//         .write_all(file.as_bytes())
//         .expect("Write failed");

//     // Pre Resolve: To find all possible dependencies
//     let config = Config::new(
//         Shell::new(),
//         env::current_dir()?,
//         format!("{}/job{}", current_path.to_str().unwrap(), thread_id).into(),
//     );
//     let mut ws = Workspace::new(&Path::new(&current_toml_path), &config).unwrap();
//     let mut registry = PackageRegistry::new(ws.config()).unwrap();
//     let requested_targets = Vec::new();
//     let requested_kinds = CompileKind::from_requested_targets(ws.config(), &requested_targets)?;
//     let target_data = RustcTargetData::new(&ws, &requested_kinds)?;
//     let specs = PackageIdSpec::query_str("dep", ws.members().map(|pkg| pkg.package_id()))?;
//     let specs = [PackageIdSpec::from_package_id(specs)];

//     let ws_resolve = ops::resolve_ws_with_opts(
//         &ws,
//         &target_data,
//         &requested_kinds,
//         &CliFeatures::new_all(true),
//         &specs,
//         HasDevUnits::No,
//         ForceAllTargets::Yes,
//     )?;
//     let package_map: HashMap<PackageId, &Package> = ws_resolve
//         .pkg_set
//         .packages()
//         .map(|pkg| (pkg.package_id(), pkg))
//         .collect();

//     // Double Resolve: Stripe unuseful packages.
//     // Configurations
//     let cli_features = CliFeatures::new_all(true);
//     let packages = Packages::Default;
//     let target = Target::Host;
//     let mut edge_kinds = HashSet::new();
//     edge_kinds.insert(EdgeKind::Dep(DepKind::Normal));
//     edge_kinds.insert(EdgeKind::Dep(DepKind::Build));
//     // edge_kinds.insert(EdgeKind::Dep(DepKind::Development));
//     let invert = vec![];
//     let pkgs_to_prune = vec![];
//     let prefix = Prefix::Indent;
//     let no_dedupe = false;
//     let duplicates = false;
//     let charset = Charset::Utf8;
//     let format = "{p}".to_string();
//     let graph_features = false;
//     let max_display_depth = u32::MAX;
//     let no_proc_macro = false;
//     // Dependency Strip
//     let opts = TreeOptions {
//         cli_features,
//         packages,
//         target,
//         edge_kinds,
//         invert,
//         pkgs_to_prune,
//         prefix,
//         no_dedupe,
//         duplicates,
//         charset,
//         format,
//         graph_features,
//         max_display_depth,
//         no_proc_macro,
//     };
//     let mut g = graph::build(
//         &ws,
//         &ws_resolve.targeted_resolve,
//         &ws_resolve.resolved_features,
//         &specs,
//         &CliFeatures::new_all(false),
//         &target_data,
//         &requested_kinds,
//         package_map,
//         &opts,
//     )?;

//     match &g.nodes[0] {
//         Node::Package{package_id, features, ..} => {
//             println!("node0: {:#?}", package_id) // Final dependency graph
//         },
//         _ => (),
//     };

//     // if let Node::Package(pack) = g.nodes[0] {
//     //     println!("node0: {:#?}", pack); // Final dependency graph
        
//     // }
//     // println!("node0: {:#?}", g.nodes[0]); // Final dependency graph
//     // println!("nodes: {:#?}", g.nodes); // Final dependency graph
//     // println!("{:#?}", g.edges); // Final dependency graph

//     // for node in g.nodes {
//     //     println!("{:#?}", node); // Final dependency graph
//     // }

//     // 3. Translate version info into id.
//     let mut map = HashMap::new();
//     let mut set = HashSet::new();
//     for node in &g.nodes {
//         let pkg = match &node {
//             Node::Package{package_id, features, ..} => {
//                 package_id
//             },
//             _ => continue,
//         };
//         map.insert(
//             (pkg.name().to_string(), pkg.version().to_string()),
//             get_version_by_name_version_test(
//                 Arc::clone(&ver_name_table),
//                 &pkg.name().to_string(),
//                 &pkg.version().to_string(),
//             )?,
//         );
//     }
//     // println!("{:#?}", map);

//     // 4. Resolve the dep tree.
//     // let root = g.package_id_for_index(0);
//     // let mut v = VecDeque::new();
//     // let mut level = 1;
//     // v.extend([Some(root), None]);

//     // while let Some(next) = v.pop_front() {
//     //     if let Some(pkg) = next {
//     //         for (pkg, _) in resolve.deps(pkg) {
//     //             set.insert((
//     //                 map[&(pkg.name().to_string(), pkg.version().to_string())],
//     //                 level,
//     //             ));
//     //             v.push_back(Some(pkg));
//     //         }
//     //     } else {
//     //         level += 1;
//     //         if !v.is_empty() {
//     //             v.push_back(None)
//     //         }
//     //     }
//     // }
//     let mut dependencies:HashMap<String, HashSet<String>> = HashMap::new();
//     let mut traversed = HashSet::<usize>::new();
//     let edges_vec = &g.edges;
//     let root = 0;
//     let mut v = VecDeque::new();
//     let mut level = 1;
//     v.extend([Some(0), None]);

//     while let Some(next) = v.pop_front() {
//         if let Some(pkg) = next {
//             if !traversed.contains(&pkg){
//                 traversed.insert(pkg);
//             }
//             // print!("{} -> ", pkg);
//             for edges in edges_vec {
//                 for (dep_type, deps) in &edges.0 {
//                     for dep in deps {
//                         // print!("{}, ", dep);
//                         let pkg = g.package_id_for_index(*dep);
//                         if !traversed.contains(dep) && !set.contains(&(
//                             map[&(pkg.name().to_string(), pkg.version().to_string())],
//                             level,
//                         )) {
//                             v.push_back(Some(*dep));
//                         }
//                         else{
//                             set.insert((
//                                 map[&(pkg.name().to_string(), pkg.version().to_string())],
//                                 level,
//                             ));
//                             let crate_name = dependencies.entry(pkg.name().to_string()).or_insert(HashSet::new());
//                             (*crate_name).insert(pkg.version().to_string());
//                         }
//                     }
//                 }
//             }
//             // print!("\n");
//         } else {
//             level += 1;
//             println!("level:{}", level);
//             if !v.is_empty() {
//                 v.push_back(None)
//             }
//         }
//     }
//     // println!("{:#?}", set);
//     let path_string = format!("{}-{}.csv", name, num);
//     write_dependency_file_sorted(path_string, &dependencies);

//     Ok(())
// }