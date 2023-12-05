/// Complete Deps
/// This crate is intended to resolve transitive dependencies without remove duplicate dependencies.
/// If dependent packages are introduced several times, we record them all rather than compressing the content.
/// The dependency 



extern crate anyhow;
extern crate crossbeam;
extern crate simplelog;

use anyhow::Error;
use rust_deps::format_virt_toml_file;
use rust_deps::util::{get_ver_name_table, VersionInfo, THREAD_DATA_SIZE};

use simplelog::*;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::OpenOptions;
use std::io::Write;

use cargo::core::registry::PackageRegistry;
use cargo::core::resolver::{CliFeatures, HasDevUnits};
use cargo::core::{Workspace, Shell};
use cargo::ops::{self};
use cargo::util::Config;

use std::env::{self, current_dir};
use std::fs::File;
use std::panic::{self, catch_unwind};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{Context, Result};
use crossbeam::channel::{self};
use log::{error, info, warn};
use postgres::{Client, NoTls};

const DB_SUFFIX: &str = "_CompleteDepth";
const LOG_FILE: &str = "./complete_deps.log";


fn main() {
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Warn,
            simplelog::Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Info,
            simplelog::Config::default(),
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .append(true)
                .open(LOG_FILE)
                .unwrap(),
        ),
    ])
    .unwrap();

    count_all_deps(20, "undone");
    // run_deps(20, "processing");
}


/// Count all versions and dependencies.
/// Run dependency resolving in `workers` threads
/// DO NOT support break-point. Should only rerun.
pub fn count_all_deps(workers: usize, status: &str) {
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

    info!(r#"\\\ !Resolving Done! ///"#);
}

fn resolve(
    thread_id: u32, 
    conn: Arc<Mutex<Client>>, 
    version_info: &VersionInfo,
    ver_name_table: Arc<HashMap<(String, String), i32>>,
) -> Result<()> {
    let version_id = version_info.version_id;
    let res = resolve_store_fulldeps_of_version(thread_id, Arc::clone(&conn), version_info, ver_name_table);

    let status = if res.is_ok() { "done" } else { "fail" };

    update_process_status(Arc::clone(&conn), version_id, status);

    res
}



/// Resolve version's dependencies and store them into db.
fn resolve_store_fulldeps_of_version(
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
    // `map`: (name, version) -> version_id
    let mut map = HashMap::new();
    let mut set = HashSet::new();
    for pkg in resolve.iter() {
        map.insert(
            (pkg.name().to_string(), pkg.version().to_string()),
            get_version_by_name_version(
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
            for (dep_pkg, _) in resolve.deps(pkg) {
                set.insert((
                    map[&(dep_pkg.name().to_string(), dep_pkg.version().to_string())], // version_to
                    map[&(pkg.name().to_string(), pkg.version().to_string())], // version_parent
                    level, // dep_level

                ));
                v.push_back(Some(dep_pkg));
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
        for (version_to, version_parent, level) in &set {
            query.push_str(&format!("({}, {}, {}, {}),", version_id, version_to, version_parent, level,));
        }
        query.pop();
        query.push(';');
        conn.lock().unwrap().query(&query, &[]).unwrap_or_default();
    }

    Ok(())
}


/// Different from `utils` library same-name function,
/// we customize DB tables with complete info stored.
/// Now, the only difference is the dep_version has one more attribute `version_parent`.
pub fn prebuild_db_table(conn: Arc<Mutex<Client>>){
    conn.lock()
    .unwrap()
    .query(
        &format!(r#"CREATE TABLE IF NOT EXISTS dep_version{DB_SUFFIX}(
                    version_from INT,
                    version_to INT,
                    version_parent INT,
                    dep_level INT
                    )"#),
        &[],
    )
    .unwrap();
    conn.lock()
        .unwrap()
        .query(
            &format!(r#"CREATE TABLE IF NOT EXISTS public.dep_errors{DB_SUFFIX}
            (
                ver integer,
                is_panic boolean,
                error text COLLATE pg_catalog."default"
            )"#),
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

pub fn get_version_by_name_version(table: Arc<HashMap<(String, String), i32>>, name: &str, version: &str) -> Result<i32> {
    Ok(
        table.get(&(name.to_string(), version.to_string()))
            .context("Can't get version_id")?
            .clone())
}


#[cfg(test)]
mod test{
    use std::io::{self, Write};

    use super::*;

    #[test]
    fn test_complete_resolve() -> io::Result<()> {
        let conn = Arc::new(Mutex::new(
            Client::connect(
                "host=localhost port=5432 dbname=crates user=postgres password=postgres",
                NoTls,
            )
            .unwrap(),
        ));
        let ver_name_table = Arc::new(get_ver_name_table(Arc::clone(&conn)));

        let version_id = 12345678; // fake
        let thread_id = 999; // fake
        let name = String::from("rand");
        let num = String::from("0.6.0");
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
        let ws = Workspace::new(&Path::new(&current_toml_path), &config).unwrap();
        let mut registry = PackageRegistry::new(ws.config()).unwrap();
        let resolve = ops::resolve_with_previous(
            &mut registry,
            &ws,
            &CliFeatures::new_all(true),
            HasDevUnits::No,
            None,
            None,
            &[],
            true,
        )
        .unwrap();

        // Find all `features` including user-defined and optional dependency
        if let Ok(res) = resolve.query(&format!("{}:{}", name, num)) {
            for feature in resolve.summary(res).features().keys() {
                features.push(feature.as_str());
            }
        } else {
            println!("NO RES");
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
        println!("Resolve: {:#?}", resolve);

        // 3. Start Formatting Resolve and store into DB
        // `map`: (name, version) -> version_id
        let mut map = HashMap::new();
        let mut set = HashSet::new();
        for pkg in resolve.iter() {
            map.insert(
                (pkg.name().to_string(), pkg.version().to_string()),
                get_version_by_name_version(
                    Arc::clone(&ver_name_table),
                    &pkg.name().to_string(),
                    &pkg.version().to_string(),
                ).unwrap(),
            );
        }
        println!("{:#?}", map);

        // Resolve the dep tree.
        let root = resolve.query(&name).unwrap();
        let mut v = VecDeque::new();
        let mut level = 1;
        v.extend([Some(root), None]);

        while let Some(next) = v.pop_front() {
            if let Some(pkg) = next {
                for (dep_pkg, _) in resolve.deps(pkg) {
                    if set.contains(&(
                        format!("{} {}", pkg.name().to_string(), pkg.version().to_string()), // version_parent
                        format!("{} {}", dep_pkg.name().to_string(), dep_pkg.version().to_string()), // version_to
                        level, // dep_level
                    )) {
                        print!("Error:{} {} {} ", pkg.name().to_string(), pkg.version().to_string(), level);
                        println!("{} {} {}", dep_pkg.name().to_string(), dep_pkg.version().to_string(), level);
                    }
                    set.insert((
                        format!("{} {}", pkg.name().to_string(), pkg.version().to_string()), // version_parent
                        format!("{} {}", dep_pkg.name().to_string(), dep_pkg.version().to_string()), // version_to
                        level, // dep_level

                    ));
                    v.push_back(Some(dep_pkg));
                }
            } else {
                level += 1;
                if !v.is_empty() {
                    v.push_back(None)
                }
            }
        }

        // Store dep info into DB.
        // if !set.is_empty() {
        //     let mut query = format!("INSERT INTO dep_version{DB_SUFFIX} VALUES");
        //     for (version_to, version_parent, level) in &set {
        //         query.push_str(&format!("({}, {}, {}, {}),", version_id, version_to, version_parent, level,));
        //     }
        //     query.pop();
        //     query.push(';');
        // }
        println!("{:#?}", &set);

        Ok(())
    }
}