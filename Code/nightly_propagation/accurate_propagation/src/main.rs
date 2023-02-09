
use cargo::core::registry::PackageRegistry;
use cargo::core::resolver::{CliFeatures, HasDevUnits};
use cargo::core::{ Shell,Workspace};
use cargo::ops;
use cargo::util::Config;

use std::env::{self, current_dir};
use std::fs::File;
use std::io::{Write};
use std::panic::{self, catch_unwind};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

use simplelog::*;
use std::fs::OpenOptions;
use anyhow::{Result};
use crossbeam::channel::{self};
use log::{error, info, warn};
use postgres::{Client, NoTls};
use pbr::MultiBar;

struct FeatureInfo {
    version_id: i32,
    name: String,
    num: String,
    feature: String,
    nightly_feature: String,
}

struct VersionInfo {
    version_id: i32,
    name: String,
    num: String,
    feature_relation: Vec<FeatureInfo>,
}



const THREAD_DATA_SIZE: usize = 20;

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
                .open("./accurate_feature.log")
                .unwrap(),
        ),
    ])
    .unwrap();

    run_deps(THREAD_DATA_SIZE);
}



/// Main Operation
/// Run dependency resolving in `workers` threads
/// Only process crates whose status = `status`
pub fn run_deps(workers: usize) {

    let conn = Arc::new(Mutex::new(
        Client::connect(
            "host=localhost dbname=crates user=postgres password=postgres",
            NoTls,
        )
        .unwrap(),
    ));
    println!("DB Prebuild and Data generation");
    let unresolved_crates = find_unresolved_crates(Arc::clone(&conn));
    let len = unresolved_crates.len();
    let mut count = 0;

    println!("Creating Channel");
    // Create channel
    let (tx, rx) = channel::bounded(2 * workers);

    // Create threads
    let mut handles = vec![];
    for i in 0..workers {
        let rx = rx.clone();

        handles.push(thread::spawn(move || {
            // Set panic hook and store into DB
            let old_hook = panic::take_hook();
            panic::set_hook({
                Box::new(move |info| {
                    error!("Thread {}: panic, {}", i, info);
                })
            });
            catch_unwind(|| {
                //  MAIN OPERATION: Dependency Resolution
                let conn = Arc::new(Mutex::new(
                    Client::connect(
                        "host=localhost dbname=crates user=postgres password=postgres",
                        NoTls,
                    )
                    .unwrap(),
                ));
                while let Ok(v) = rx.recv() {
                    let v = v as VersionInfo;
                    let version_id = v.version_id;
                    if let Err(e) = resolve(i as u32, Arc::clone(&conn), v) {
                        warn!(
                            "Resolve version {} fails, due to error: {}",
                            version_id, e
                        );
                    } else {
                        info!("Thread {}: Done version - {}", i, version_id);
                    }
                }
            })
            .unwrap_or_default();
            panic::set_hook(old_hook); // Must after catch_unwind
        }));
    }

    for (version_id, name, num) in unresolved_crates {
        let conn = Arc::clone(&conn);
        let query = format!(
            r#"SELECT version_to, name, num, feature, nightly
            FROM feature_propagation_indir_relation INNER JOIN versions_with_name
            ON version_to = id WHERE ver = {}"#,
            version_id
        );
        let rows = conn.lock().unwrap().query(&query, &[]).unwrap();
        let features_info: Vec<FeatureInfo> = rows
            .iter()
            .map(|row| FeatureInfo {
                version_id: row.get(0),
                name: row.get(1),
                num: row.get(2),
                feature: row.get(3),
                nightly_feature: row.get(4),
            })
            .collect();
        let version_info = VersionInfo{
            version_id,
            name,
            num,
            feature_relation: features_info,
        };
        tx.send(version_info).unwrap();
        count += 1;
        info!("Status: {}/{}", count, len);
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
fn resolve(thread_id: u32, conn: Arc<Mutex<Client>>, version_info: VersionInfo) -> Result<()> {
    let version_id = version_info.version_id;
    let res = resolve_store_deps_of_version(thread_id, Arc::clone(&conn), version_info);
    let status = if res.is_ok() { "done" } else { "fail" };
    update_process_status(Arc::clone(&conn), version_id, status);
    res
}

fn update_process_status(conn: Arc<Mutex<Client>>, version_id: i32, status: &str){
    // warn!("update status");
    conn.lock()
        .unwrap()
        .query(
            &format!(
                "UPDATE feature_propagation_ver_status SET status = '{}' WHERE ver = '{}';",
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
    version_info: VersionInfo,
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

    // 3. Find Nightly Feature Dependency
    let mut nightly_features:Vec<FeatureInfo> = Vec::new();
    for feature_info in version_info.feature_relation{
        let target_version = format!("{}:{}", feature_info.name, feature_info.num);
        let query_feature = &feature_info.feature;
        if let Ok(res) = resolve.query(&target_version) {
            for feature in resolve.features(res) {
                if feature == query_feature.as_str(){
                    nightly_features.push(feature_info);
                    break;
                }
            }
        }
    }

    // 4. Store results into DB
    if !nightly_features.is_empty() {
        let mut query = String::from("INSERT INTO dep_version_feature VALUES");
        for nightly_feature in nightly_features {
            query.push_str(&format!("({}, {}, '{}', '{}'),", 
            version_id, 
            nightly_feature.version_id, 
            nightly_feature.feature,
            nightly_feature.nightly_feature,
        ));
        }
        query.pop();
        query.push(';');
        conn.lock().unwrap().query(&query, &[]).unwrap_or_default();
    }

    Ok(())
}

fn find_unresolved_crates(conn: Arc<Mutex<Client>>) -> Vec<(i32, String, String)> {
    // Find possible Nightly feature  dependents
    conn.lock()
    .unwrap()
    .query(
        r#"CREATE TABLE IF NOT EXISTS feature_propagation_indir_relation AS
            (WITH tmp AS(
                SELECT id, SUBSTRING(conds, 11) as feature, feature as nightly 
                FROM version_feature 
                WHERE conds LIKE 'feature = %' AND feature != 'no_feature_used'
            )
            SELECT DISTINCT version_from as ver, version_to, feature, nightly 
            FROM tmp INNER JOIN dep_version ON id = version_to)"#,
        &[],
    )
    .unwrap();
    // Resolve status table
    conn.lock()
        .unwrap()
        .query(r#"
            CREATE TABLE IF NOT EXISTS feature_propagation_ver_status AS
            (WITH dep_from AS(
                SELECT DISTINCT ver FROM feature_propagation_indir_relation
            )
            SELECT ver, name, num, 'unresolved' as status
                FROM dep_from INNER JOIN versions_with_name ON ver = id)"#,
            &[],
        )
        .unwrap();
    // Resolve Results
    conn.lock()
    .unwrap()
    .query(
        r#"CREATE TABLE IF NOT EXISTS dep_version_feature(
                    version_from INT,
                    version_to INT,
                    feature VARCHAR,
                    nightly_feature VARCHAR
        )"#,
        &[],
    ).unwrap();
    let query = format!(
        "SELECT * FROM feature_propagation_ver_status WHERE status = 'unresolved'"
    );
    let row = conn.lock().unwrap().query(&query, &[]).unwrap();
    row.iter().map(|ver| {
        let version_id:i32 = ver.get(0);
        let name:&str = ver.get(1);
        let num:&str = ver.get(2);
        (version_id, name.to_string(), num.to_string())
    }
    ).collect()
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
    find_unresolved_crates(Arc::clone(&conn));
    Ok(())
}
