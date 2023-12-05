extern crate anyhow;
extern crate crossbeam;
extern crate simplelog;

use anyhow::Error;
use rust_deps::format_virt_toml_file;
use rust_deps::util::{get_ver_name_table, VersionInfo, THREAD_DATA_SIZE};

use simplelog::*;
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

use crossbeam::channel::{self};
use log::{error, info, warn};
use postgres::{Client, NoTls};



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
                .open("./count_deps.log")
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
    let conn = Arc::new(Mutex::new(
        Client::connect(
            "host=localhost dbname=crates user=postgres password=postgres",
            NoTls,
        )
        .unwrap(),
    ));
    // Create channel
    println!("Creating Channel");
    let (tx, rx) = channel::bounded(workers);

    // Create threads
    let mut handles = vec![];
    for i in 0..workers {
        let rx = rx.clone();

        handles.push(thread::spawn(move || {
            let version_count = Arc::new(Mutex::new(0));
            let deps_count = Arc::new(Mutex::new(0));
            while let Ok(versions) = rx.recv() {
                for v in versions {
                    let v = v as VersionInfo;
                    // Set panic hook
                    let old_hook = panic::take_hook();
                    panic::set_hook({
                        Box::new(move |info| {
                            let err_message = format!("{:?}", info);
                            error!(
                                "Thread {}: Panic occurs, version - {}, info:{}",
                                i, v.version_id, err_message
                            );
                        })
                    });
                    if catch_unwind(|| {
                        //  MAIN OPERATION: Dependency Resolution
                        match get_dep_count(i as u32,
                                &v) {
                            Err(e) => {
                                warn!(
                                    "Resolve version {} fails, due to error: {}",
                                    v.version_id, e
                                );
                            },
                            Ok(count) => {
                                *(deps_count.lock().unwrap()) += count;
                                *(version_count.lock().unwrap()) += 1;
                                info!("Thread {}: Version_count {}, Deps count {}",
                                     i, version_count.lock().unwrap(), deps_count.lock().unwrap());
                            }
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
    let conn = Arc::clone(&conn);
    loop{
        let mut offset = 592000;
        let query = format!(
            r#"SELECT id,crate_id,name,num FROM versions_with_name OFFSET {} LIMIT {}"#,
            offset, THREAD_DATA_SIZE
        );
        offset += THREAD_DATA_SIZE;
        let rows = conn.lock().unwrap().query(&query, &[]).unwrap();
        if rows.is_empty() {
            break;
        }
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

    std::mem::drop(tx);
    for handle in handles {
        // Unsolved problem
        if handle.join().is_err() {
            error!("!!!Thread Crash!!!")
        }
    }

    info!(r#"\\\ !Resolving Done! ///"#);
}

/// Resolve version's dependencies and store them into db.
fn get_dep_count(
    thread_id: u32,
    version_info: &VersionInfo,
) -> Result<usize, Error> {
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

    // 3. Count deps
    let mut count_dep = 0;
    for pkg in resolve.iter() {
        let dep_num = resolve.deps(pkg).count();
        let dep_name = pkg.name().to_string();
        count_dep += dep_num;
    }
    count_dep -= 1; // Remove pkg 'dep' (our virtual pkg).

    Ok(count_dep)
}