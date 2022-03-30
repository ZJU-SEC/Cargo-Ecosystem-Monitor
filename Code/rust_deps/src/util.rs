use cargo::core::registry::PackageRegistry;
use cargo::core::resolver::{CliFeatures, HasDevUnits};
use cargo::core::{Shell, Workspace};
use cargo::ops;
use cargo::util::Config;

use std::collections::{HashMap, HashSet, VecDeque};
use std::env::{self, current_dir};
use std::fs::File;
use std::io::Write;
use std::panic::{catch_unwind, self};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{Context, Result};
use crossbeam::channel::{self};
use log::{error, info, warn};
use postgres::{Client, NoTls};

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
fn store_resolve_error(conn: Arc<Mutex<Client>>, version: i32, is_panic:bool, message: String){
    let query = format! {
        "INSERT INTO dep_errors(ver, is_panic, error) VALUES ({}, {:?}, '{}');",
        version, is_panic, message
    };
    conn.lock().unwrap().query(&query, &[]).unwrap_or_default();
}

/// Resolve version's dependencies and store them into db.
fn resolve_store_deps_of_version(
    thread_id: u32,
    conn: Arc<Mutex<Client>>,
    version_id: i32,
) -> Result<()> {
    let name = get_name_by_version_id(Arc::clone(&conn), version_id)?;
    let num = get_version_str_by_version_id(Arc::clone(&conn), version_id)?;

    let mut file = String::from(
        r#"[package]
name = "dep"
version = "0.1.0"
edition = "2021"

[dependencies]"#,
    );

    file.push_str(&format!("\n{} = \"={}\"", name, num));

    let current_path = current_dir()?;
    let mut current_toml_path = String::new();
    let dep_filename = format!("dep{}.toml", thread_id);
    current_toml_path.push_str(current_path.to_str().unwrap());
    current_toml_path.push_str("/");
    current_toml_path.push_str(&dep_filename);

    File::create(&current_toml_path)?
        .write_all(file.as_bytes())
        .expect("Write failed");

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
        HasDevUnits::Yes,
        None,
        None,
        &[],
        true,
    )?;

    // println!("{:#?}", resolve);
    // TODO: Preprocess resolve
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
    if ! set.is_empty(){
        let mut query = String::from("INSERT INTO dep_version VALUES");
        for (version_to, level) in set {
            query.push_str(&format!(
                "({}, {}, {}),",
                version_id, version_to, level,
            ));
        }
        query.pop();
        query.push(';');
        conn.lock().unwrap().query(&query, &[]).unwrap_or_default();
    }
    Ok(())
}

/// Run dependency resolving in `workers` threads
pub fn run_deps(workers: usize) {
    let conn = Arc::new(Mutex::new(
        Client::connect(
            "host=localhost dbname=crates user=postgres password=postgres",
            NoTls,
        )
        .unwrap(),
    ));
    conn.lock()
        .unwrap()
        .query(
            r#"CREATE TABLE dep_version(
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

    // Decide start point
    let mut offset = 0i64;//84472;offset168000,176000;191000;241872
    let res = conn
        .lock()
        .unwrap()
        .query(
            "SELECT version_from FROM dep_version ORDER BY version_from desc LIMIT 1",
            &[],
        )
        .unwrap();

    if let Some(last) = res.first() {
        let last: i32 = last.get(0);
        let query = format!(
            "with max_crate as (SELECT MAX(crate_id) 
            FROM dep_version INNER JOIN versions on dep_version.version_from=versions.id) 
            SELECT COUNT(versions) FROM versions WHERE versions.crate_id<ANY(SELECT max FROM max_crate)"
        );

        offset = conn
            .lock()
            .unwrap()
            .query(&query, &[])
            .unwrap()
            .first()
            .unwrap()
            .get(0);
    }

    info!("Starting offset: {}", offset);

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
                    let old_hook = panic::take_hook();
                    panic::set_hook({
                        let conn_copy = conn.clone();
                        Box::new(move |info| {
                            let err_message = format!("{:?}",info);
                            store_resolve_error(Arc::clone(&conn_copy),
                                    v, true, err_message);
                        })
                    });
                    if catch_unwind(|| {
                        if let Err(e) =
                            resolve_store_deps_of_version(i as u32, Arc::clone(&conn), v)
                        {
                            warn!("Resolve version {} fails, due to error: {}", v, e);
                            store_resolve_error(Arc::clone(&conn), v, false, format!("{:?}",e));
                        } else {
                            info!("Thread {}: Done version - {}", i, v);
                        }
                    })
                    .is_err()
                    {
                        error!("Thread {}: Panic occurs, version - {}", i, v);
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
            "SELECT id FROM versions ORDER BY crate_id asc LIMIT 250 OFFSET {}",
            offset
        );
        let rows = conn.lock().unwrap().query(&query, &[]).unwrap();

        if rows.is_empty() {
            break;
        } else {
            let v: Vec<i32> = rows.iter().map(|version| version.get(0)).collect();
            tx.send(v).expect("Send task error!");
        }
        offset += 250;
        warn!("OFFSET = {}", offset);
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
