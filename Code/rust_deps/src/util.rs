use cargo::core::registry::PackageRegistry;
use cargo::core::resolver::{CliFeatures, HasDevUnits};
use cargo::core::Workspace;
use cargo::ops;
use cargo::util::Config;

use anyhow::{Context, Result};
use log::{info, warn};
use postgres::{Client, NoTls};
use std::collections::VecDeque;
use std::env::current_dir;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

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

/// Resolve version's dependencies and store them into db.
fn resolve_store_deps_of_version(
    conn: Arc<Mutex<Client>>,
    version_id: i32,
    dep_filename: &str,
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

    let current_path = current_dir().unwrap();
    let mut current_toml_path = String::new();
    current_toml_path.push_str(current_path.to_str().unwrap());
    current_toml_path.push_str("/");
    current_toml_path.push_str(dep_filename);

    File::create(&current_toml_path)
        .unwrap()
        .write_all(file.as_bytes())
        .expect("Write failed");

    let config = Config::default().unwrap();
    let ws = Workspace::new(&Path::new(&current_toml_path), &config).unwrap();

    let mut registry = PackageRegistry::new(ws.config()).unwrap();
    let mut resolve = ops::resolve_with_previous(
        &mut registry,
        &ws,
        &CliFeatures::new_all(true),
        HasDevUnits::Yes,
        None,
        None,
        &[],
        true,
    )?;

    let root = resolve.query(&name)?;
    let mut v = VecDeque::new();
    let mut level = 1;
    v.extend([Some(root), None]);

    while let Some(next) = v.pop_front() {
        if let Some(pkg) = next {
            for (pkg, _) in resolve.deps(pkg) {
                let query = format!(
                    "INSERT INTO dep_version VALUES({}, {}, {})",
                    version_id,
                    get_version_by_name_version(
                        Arc::clone(&conn),
                        &pkg.name().to_string(),
                        &pkg.version().to_string(),
                    )?,
                    level
                );
                conn.lock().unwrap().query(&query, &[]).unwrap_or_default();
                v.push_back(Some(pkg));
            }
        } else {
            level += 1;
            if !v.is_empty() {
                v.push_back(None)
            }
        }
    }
    Ok(())
}

fn run_one_pass(conn: Arc<Mutex<Client>>, versions: Arc<Vec<i32>>, jobs: usize) {
    let mut handles = vec![];

    for i in 0..jobs {
        let conn = Arc::clone(&conn);
        let version = Arc::clone(&versions);
        let filename = format!("dep{}.toml", i);

        handles.push(thread::spawn(move || {
            let mut index = i as usize;
            while index < version.len() {
                let v = version[index];
                if let Err(e) = resolve_store_deps_of_version(Arc::clone(&conn), v, &filename) {
                    warn!("{}", e);
                } else {
                    info!("Done version - {}", v);
                }
                index += jobs;
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

pub fn run_deps(jobs: usize) {
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

    // Get versions
    let mut offset = 0u32;
    loop {
        let conn = Arc::clone(&conn);
        let query = format!(
            "SELECT id FROM versions ORDER BY crate_id asc LIMIT 1000 OFFSET {}",
            offset
        );
        let rows = conn.lock().unwrap().query(&query, &[]).unwrap();
        if rows.is_empty() {
            break;
        } else {
            let v: Vec<i32> = rows.iter().map(|version| version.get(0)).collect();
            run_one_pass(conn, Arc::new(v), jobs);
        }
        offset += 1000;
    }

    println!("Resolving Done!");
}
