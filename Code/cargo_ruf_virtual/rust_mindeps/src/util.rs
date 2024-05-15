use cargo::core::registry::PackageRegistry;
use cargo::core::resolver::{CliFeatures, HasDevUnits};
use cargo::core::{Shell, Workspace};
use cargo::ops;
use cargo::util::Config;

use std::collections::{HashMap, HashSet, VecDeque};
use std::env::{self, current_dir};
use std::fs::File;
use std::io::Write;
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

// Suffix of DB, used for test and other purposes.
// It is empty by default. If it is not, it is not used for general purposes.
// pub const DB_SUFFIX: &str = "_minimal";
pub const DB_SUFFIX: &str = "_normal";


/// Main Operation
/// Run dependency resolving in `workers` threads
/// Only process crates whose status = `status`
pub fn run_deps(workers: usize, status: &str) {
    if status == "processing" {
        panic!(
            "If you specify undone, it will automatically 
        process crates whose status is 'processing'"
        )
    }
    if status != "undone" && status != "fail" {
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
                        if let Err(e) =
                            resolve(i as u32, Arc::clone(&conn), &v, Arc::clone(&ver_name_table))
                        {
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
                        store_resolve_error(Arc::clone(&conn), v.version_id, true, "".to_string());
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
                SELECT version_id FROM ruf_audit_dep_process_status{DB_SUFFIX} WHERE status='{}' ORDER BY version_id asc LIMIT {}
                )"#,
            status, THREAD_DATA_SIZE
        );

        let rows = conn.lock().unwrap().query(&query, &[]).unwrap();

        if rows.is_empty() {
            break;
        } else {
            let query = format!(
                r#"UPDATE ruf_audit_dep_process_status{DB_SUFFIX} SET status='processing' WHERE version_id IN (
                    SELECT version_id FROM ruf_audit_dep_process_status{DB_SUFFIX} WHERE status='{}' ORDER BY version_id asc LIMIT {}
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
    let res =
        resolve_store_deps_of_version(thread_id, Arc::clone(&conn), version_info, ver_name_table);

    let status = if res.is_ok() { "done" } else { "fail" };

    update_process_status(Arc::clone(&conn), version_id, status);

    res
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
    let mut config = Config::new(
        Shell::new(),
        env::current_dir()?,
        format!("{}/job{}", current_path.to_str().unwrap(), thread_id).into(),
    );

    // config.configure(
    //     0,
    //     false,
    //     None,
    //     false,
    //     false,
    //     false,
    //     &None,
    //     &["minimal-versions".to_string()],
    //     &[],
    // )?;

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

    let mut config = Config::new(
        Shell::new(),
        env::current_dir()?,
        format!("{}/job{}", current_path.to_str().unwrap(), thread_id).into(),
    );

    // config.configure(
    //     0,
    //     false,
    //     None,
    //     false,
    //     false,
    //     false,
    //     &None,
    //     &["minimal-versions".to_string()],
    //     &[],
    // )?;

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
        let mut query = format!("INSERT INTO ruf_audit_dep_version{DB_SUFFIX} VALUES");
        for (version_to, level) in set {
            query.push_str(&format!("({}, {}, {}),", version_id, version_to, level,));
        }
        query.pop();
        query.push(';');
        conn.lock().unwrap().query(&query, &[]).unwrap_or_default();
    }

    Ok(())
}

fn prebuild_db_table(conn: Arc<Mutex<Client>>) {
    conn.lock()
        .unwrap()
        .query(
            &format!(
                r#"CREATE TABLE IF NOT EXISTS ruf_audit_dep_version{DB_SUFFIX}(
                    version_from INT,
                    version_to INT,
                    dep_level INT,
                    UNIQUE(version_from, version_to, dep_level))"#
            ),
            &[],
        )
        .unwrap();

    conn.lock()
        .unwrap()
        .query(
            &format!(
                r#"CREATE TABLE IF NOT EXISTS ruf_audit_dep_errors{DB_SUFFIX}
            (
                ver integer,
                is_panic boolean,
                error text COLLATE pg_catalog."default",
                UNIQUE (ver, is_panic, error)
            )"#
            ),
            &[],
        )
        .unwrap();

    conn.lock().unwrap().query(&format!(r#"CREATE VIEW versions_with_name as (
        SELECT versions.*, crates.name FROM versions INNER JOIN crates ON versions.crate_id = crates.id
        )"#), &[]).unwrap_or_default();

    // Crate resolution process
    conn.lock()
        .unwrap()
        .query(
            &format!(
                r#"CREATE TABLE IF NOT EXISTS ruf_audit_dep_process_status{DB_SUFFIX}
            (
                version_id INT,
                status VARCHAR
            )"#
            ),
            &[],
        )
        .unwrap();

    // Check if table is empty
    if conn
        .lock()
        .unwrap()
        .query(
            &format!("SELECT * FROM ruf_audit_dep_process_status{DB_SUFFIX} LIMIT 1"),
            &[],
        )
        .unwrap()
        .first()
        .is_none()
    {
        conn.lock()
            .unwrap()
            .query(
                &format!(
                    "
                INSERT INTO ruf_audit_dep_process_status{DB_SUFFIX}(
                    SELECT DISTINCT id, 'undone'
                    FROM tmp_ruf_impact
                    WHERE status = 'removed' OR status = 'unknown'
                )"
                ),
                &[],
            )
            .unwrap();
    } else {
        let query = format!(
            r#"UPDATE ruf_audit_dep_process_status{DB_SUFFIX} SET status='undone' WHERE version_id IN (
                SELECT version_id FROM ruf_audit_dep_process_status{DB_SUFFIX} WHERE status='processing'
            )"#
        );
        conn.lock().unwrap().query(&query, &[]).unwrap();
    }
}

fn get_ver_name_table(conn: Arc<Mutex<Client>>) -> HashMap<(String, String), i32> {
    let rows = conn
        .lock()
        .unwrap()
        .query(
            &format!(r#"SELECT id, crate_id, num ,name FROM versions_with_name"#),
            &[],
        )
        .unwrap();
    // <(name, num), version_id>
    let mut ver_name_table: HashMap<(String, String), i32> = HashMap::new();
    for ver in rows {
        let name: String = ver.get(3);
        let num: String = ver.get(2);
        let version_id: i32 = ver.get(0);
        ver_name_table.entry((name, num)).or_insert(version_id);
    }
    ver_name_table
}

// Although it is marked as `test`, it performs better than the original code. We will do code refactor later.
pub fn get_version_by_name_version_test(
    table: Arc<HashMap<(String, String), i32>>,
    name: &str,
    version: &str,
) -> Result<i32> {
    Ok(table
        .get(&(name.to_string(), version.to_string()))
        .context("Can't get version_id")?
        .clone())
}

/// Get version by name and version string
///
/// # Example
/// ```
/// store_resolve_error(conn, 362968, false, String::new("Error"));
/// ```
pub fn store_resolve_error(
    conn: Arc<Mutex<Client>>,
    version: i32,
    is_panic: bool,
    message: String,
) {
    let message = message.replace("'", "''");
    let query = format! {
        "INSERT INTO ruf_audit_dep_errors{DB_SUFFIX}(ver, is_panic, error) VALUES ({}, {:?}, '{}');",
        version, is_panic, message
    };
    conn.lock().unwrap().query(&query, &[]).expect("Store resolve error fails");
    update_process_status(Arc::clone(&conn), version, "fail");
}

fn update_process_status(conn: Arc<Mutex<Client>>, version_id: i32, status: &str) {
    conn.lock()
        .unwrap()
        .query(
            &format!(
                "UPDATE ruf_audit_dep_process_status{DB_SUFFIX} SET status = '{}' WHERE version_id = '{}';",
                status, version_id
            ),
            &[],
        )
        .expect("Update process status fails");
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
        "{} = {{version = \"={}\", features = [",
        name, version_num
    ));
    for feature in features {
        file.push_str(&format!("\"{}\",", feature));
    }
    file.push_str("]}");
    file
}
