use cargo::core::registry::PackageRegistry;
use cargo::core::resolver::{CliFeatures, HasDevUnits};
use cargo::core::Workspace;
use cargo::ops;
use cargo::util::Config;

use postgres::Client;
use std::collections::VecDeque;
use std::env::current_dir;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Get a name by crate id.
///
/// # Example
/// ```
/// let name = get_name_by_crate_id(conn, 321);
/// assert_eq!(&name, "ecs")
/// ```
///
/// # Panic
/// If the crate **DONOT** exist, function will panics.
fn get_name_by_crate_id(conn: &mut Client, crate_id: i32) -> String {
    let query = format!("SELECT name FROM crates WHERE id = {} LIMIT 1", crate_id);
    let row = conn.query(&query, &[]).unwrap();
    row.first()
        .expect(&format!(
            "Get name by crate id fails, crate id: {}",
            crate_id
        ))
        .get(0)
}

/// Get a name by version id.
///
/// # Example
/// ```
/// let name = get_name_by_version_id(conn, 6034);
/// assert_eq!(&name, "ecs")
/// ```
///
/// # Panic
/// If the crate **DONOT** exist, function will panics.
fn get_name_by_version_id(conn: &mut Client, version_id: i32) -> String {
    let query = format!(
        "SELECT crate_id FROM versions WHERE id = {} LIMIT 1",
        version_id
    );
    let row = conn.query(&query, &[]).unwrap();
    let crate_id = row
        .first()
        .expect(&format!(
            "Get name by version id fails, version id: {}",
            version_id
        ))
        .get(0);
    get_name_by_crate_id(conn, crate_id)
}

/// Get crate id by name.
///
/// # Example
/// ```
/// let crate_id = get_crate_id_by_name(conn, http);
/// assert_eq!(crate_id, 184);
/// ```
fn get_crate_id_by_name(conn: &mut Client, name: &str) -> i32 {
    let query = format!("SELECT id FROM crates WHERE name = '{}' LIMIT 1", name);
    let row = conn.query(&query, &[]).unwrap();
    row.first()
        .expect(&format!("Get crate id by name fails, name: {}", name))
        .get(0)
}

/// Get version by name and version string
///
/// # Example
/// ```
/// let version_id = get_version_by_name_version(conn, "http", "0.2.4");
/// assert_eq!(version_id, 362968);
/// ```
fn get_version_by_name_version(conn: &mut Client, name: &str, version: &str) -> i32 {
    let query = format! {
        "SELECT id FROM versions WHERE num = '{}' AND crate_id = '{}' LIMIT 1",
        version,
        get_crate_id_by_name(conn, name)
    };
    let row = conn.query(&query, &[]).unwrap();
    row.first()
        .expect(&format!(
            "Get version id by name & version fails, name:{} version: {}",
            name, version
        ))
        .get(0)
}

/// Get dependency versions by version id
pub fn get_deps_of_version(conn: &mut Client, version_id: i32) -> Vec<(i32, i32)> {
    let query = format!(
        "SELECT version_to,dep_level FROM dep_version WHERE version_from = '{}'",
        version_id
    );
    let mut res = conn.query(&query, &[]).unwrap();

    // If not resolved yet.
    if res.is_empty() {
        resolve_store_deps_of_version(conn, version_id);
        res = conn.query(&query, &[]).unwrap();
    }

    res.iter().map(|r| (r.get(0), r.get(1))).collect()
}

/// Resolve version's dependencies and store them into db.
pub fn resolve_store_deps_of_version(conn: &mut Client, version_id: i32) {
    let query = format!(
        "SELECT distinct crate_id,req FROM dependencies WHERE version_id = '{}'",
        version_id
    );

    let mut file = String::from(
        r#"[package]
name = "dep"
version = "0.1.0"
edition = "2021"

[dependencies]"#,
    );

    for row in conn.query(&query, &[]).unwrap() {
        let name = get_name_by_crate_id(conn, row.get(0));
        let req: String = row.get(1);
        file.push_str(&format!("\n{} = \"{}\"", name, req));
    }

    let current_path = current_dir().unwrap();
    let mut current_toml_path = String::new();
    current_toml_path.push_str(current_path.to_str().unwrap());
    current_toml_path.push_str("/dep.toml");

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
    )
    .unwrap();

    let root = resolve.query("dep").expect("Get root error!");
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
                        conn,
                        &pkg.name().to_string(),
                        &pkg.version().to_string(),
                    ),
                    level
                );
                conn.query(&query, &[]).unwrap_or_default();
                v.push_back(Some(pkg));
            }
        } else {
            level += 1;
            if !v.is_empty() {
                v.push_back(None)
            }
        }
    }
}
