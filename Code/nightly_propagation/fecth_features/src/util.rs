use std::fs::{create_dir, remove_dir_all, File};
use std::io::{Read, Write};
use std::panic::{self, catch_unwind};
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{anyhow, Error, Result};
use crossbeam::channel::{self};
use downloader::{Download, Downloader};
use flate2::read::GzDecoder;
use lazy_static::lazy_static;
use log::{error, warn};
use pbr::ProgressBar;
use postgres::{Client, NoTls};
use regex::Regex;
use tar::Archive;
use toml::Value;
use walkdir::WalkDir;

const THREAD_LOAD: i32 = 20;

struct VersionInfo {
    version_id: i32,
    _crate_id: i32,
    name: String,
    num: String,
}

// https://crates.io/api/v1/crates/$(crate)/$(version)/download

const RUSTC: &str = "/home/loancold/Projects/Cargo-Ecosystem-Monitor/rust/build/x86_64-unknown-linux-gnu/stage1/bin/rustc";

#[allow(unused)]
pub fn run(workers: usize, todo_status: &str) {
    let conn = Arc::new(Mutex::new(
        Client::connect(
            "host=localhost dbname=crates user=postgres password=postgres",
            NoTls,
        )
        .unwrap(),
    ));

    println!("DB Prebuild");
    prebuild_db_table(Arc::clone(&conn));

    let todo_count: i64 = conn
        .lock()
        .unwrap()
        .query(
            &format!(
                "SELECT COUNT(version_id) FROM feature_process_status WHERE status = '{}'",
                todo_status
            ),
            &[],
        )
        .unwrap()
        .first()
        .unwrap()
        .get(0);

    create_dir(&format!("on_process")).unwrap_or_default();

    let mut mb = ProgressBar::new(todo_count as u64);
    mb.format("╢▌▌░╟");
    mb.set(0);

    let (tx, rx) = channel::bounded(2 * workers);

    let mut handles = vec![];
    for i in 0..workers {
        let rx = rx.clone();
        let conn = Arc::clone(&conn);
        let home = format!("on_process/job{}", i);

        create_dir(&home).unwrap_or_default();

        // Start Fetching
        handles.push(thread::spawn(move || {
            let old_hook = panic::take_hook();
            panic::set_hook({
                Box::new(move |info| {
                    error!("Thread {}: panic, {}", i, info);
                })
            });

            catch_unwind(|| {
                let mut downloader = Downloader::builder()
                    .download_folder(Path::new(&home))
                    .parallel_requests(1)
                    .build()
                    .expect("Fatal Error, build downloader fails!");

                while let Ok(version_info) = rx.recv() {
                    create_dir(&home).unwrap_or_default();

                    extract_info(
                        Arc::clone(&conn),
                        Some(&mut downloader),
                        version_info,
                        &home,
                    );

                    remove_dir_all(&home).unwrap_or_default();
                }
            })
            .unwrap_or_default();

            panic::set_hook(old_hook);
        }));
    }

    loop {
        let conn = Arc::clone(&conn);
        let query = format!(
            r#"SELECT id,crate_id,name,num FROM versions_with_name WHERE id in (
                SELECT version_id FROM feature_process_status WHERE status='{}' ORDER BY version_id asc LIMIT {}
                )"#,
            todo_status, THREAD_LOAD
        );

        let rows = conn.lock().unwrap().query(&query, &[]).unwrap();

        if rows.is_empty() {
            break;
        } else {
            let query = format!(
                "UPDATE feature_process_status SET status='processing' WHERE version_id IN (
                    SELECT version_id FROM feature_process_status WHERE status='{}' ORDER BY version_id asc LIMIT {}
                )",
                todo_status, THREAD_LOAD
            );

            conn.lock().unwrap().query(&query, &[]).unwrap();

            let versions: Vec<VersionInfo> = rows
                .iter()
                .map(|row| VersionInfo {
                    version_id: row.get(0),
                    _crate_id: row.get(1),
                    name: row.get(2),
                    num: row.get(3),
                })
                .collect();

            mb.add(versions.len() as u64);

            tx.send(versions).expect("Fatal Error, send message fails!");
        }
    }

    std::mem::drop(tx);

    mb.finish();

    for handle in handles {
        // Unsolved problem
        if handle.join().is_err() {
            error!("!!!Thread Crash!!!")
        }
    }

    println!(r#"\\\ Done! ///"#)
}

#[allow(unused)]
pub fn run_offline(workers: usize, todo_status: &str, home: &str) {
    let conn = Arc::new(Mutex::new(
        Client::connect(
            "host=localhost dbname=crates user=postgres password=postgres",
            NoTls,
        )
        .unwrap(),
    ));

    println!("DB Prebuild");
    prebuild_db_table(Arc::clone(&conn));

    let todo_count: i64 = conn
        .lock()
        .unwrap()
        .query(
            &format!(
                "SELECT COUNT(version_id) FROM feature_process_status WHERE status = '{}'",
                todo_status
            ),
            &[],
        )
        .unwrap()
        .first()
        .unwrap()
        .get(0);

    let mut mb = ProgressBar::new(todo_count as u64);
    mb.format("╢▌▌░╟");
    mb.set(0);

    let (tx, rx) = channel::bounded(2 * workers);

    let mut handles = vec![];
    for i in 0..workers {
        let rx = rx.clone();
        let conn = Arc::clone(&conn);
        let home = home.to_string();

        // Start Fetching
        handles.push(thread::spawn(move || {
            let old_hook = panic::take_hook();
            panic::set_hook({
                Box::new(move |info| {
                    error!("Thread {}: panic, {}", i, info);
                })
            });

            catch_unwind(|| {
                while let Ok(version_info) = rx.recv() {
                    extract_info(Arc::clone(&conn), None, version_info, &home);
                }
            })
            .unwrap_or_default();

            panic::set_hook(old_hook);
        }));
    }

    loop {
        let conn = Arc::clone(&conn);
        let query = format!(
            r#"SELECT id,crate_id,name,num FROM versions_with_name WHERE id in (
                SELECT version_id FROM feature_process_status WHERE status='{}' ORDER BY version_id asc LIMIT {}
                )"#,
            todo_status, THREAD_LOAD
        );

        let rows = conn.lock().unwrap().query(&query, &[]).unwrap();

        if rows.is_empty() {
            break;
        } else {
            let query = format!(
                "UPDATE feature_process_status SET status='processing' WHERE version_id IN (
                    SELECT version_id FROM feature_process_status WHERE status='{}' ORDER BY version_id asc LIMIT {}
                )",
                todo_status, THREAD_LOAD
            );

            conn.lock().unwrap().query(&query, &[]).unwrap();

            let versions: Vec<VersionInfo> = rows
                .iter()
                .map(|row| VersionInfo {
                    version_id: row.get(0),
                    _crate_id: row.get(1),
                    name: row.get(2),
                    num: row.get(3),
                })
                .collect();

            mb.add(versions.len() as u64);

            tx.send(versions).expect("Fatal Error, send message fails!");
        }
    }

    std::mem::drop(tx);

    mb.finish();

    for handle in handles {
        // Unsolved problem
        if handle.join().is_err() {
            error!("!!!Thread Crash!!!")
        }
    }

    println!(r#"\\\ Done! ///"#)
}

fn extract_info(
    conn: Arc<Mutex<Client>>,
    downloader: Option<&mut Downloader>,
    versions: Vec<VersionInfo>,
    home: &str,
) {
    if let Some(downloader) = downloader {
        let success = fetch_version(Arc::clone(&conn), downloader, versions);
        deal_version(Arc::clone(&conn), success, home, false);
    } else {
        deal_version(Arc::clone(&conn), versions, home, true);
    };
}

fn fetch_version(
    conn: Arc<Mutex<Client>>,
    downloader: &mut Downloader,
    mut versions: Vec<VersionInfo>,
) -> Vec<VersionInfo> {
    let mut dls = vec![];
    let mut fail_id = vec![];

    for v in &versions {
        dls.push(
            Download::new(&format!(
                "https://crates.io/api/v1/crates/{}/{}/download",
                v.name, v.num
            ))
            .file_name(Path::new(&format!("{}-{}.tgz", v.name, v.num))),
        );
    }

    // Download fail info
    for (id, err) in downloader
        .download(&dls)
        .expect("downloader broken")
        .iter()
        .enumerate()
        .filter(|(_, res)| res.is_err())
    {
        // TODO: fix bug on id-shift
        let fail = &versions[id];
        fail_id.push(id);

        store_fails_info(
            Arc::clone(&conn),
            fail.version_id,
            &fail.name,
            &format!("Donwload fails: {}", err.as_ref().unwrap_err()),
        );
    }

    versions
        .into_iter()
        .enumerate()
        .filter(|(id, _)| !fail_id.contains(id))
        .map(|(_, v)| v)
        .collect()
}

fn deal_version(conn: Arc<Mutex<Client>>, versions: Vec<VersionInfo>, home: &str, offline: bool) {
    for (res, v) in versions
        .iter()
        .map(|v| deal_one_version(Arc::clone(&conn), v, home, offline))
        .collect::<Vec<Result<(), Error>>>()
        .into_iter()
        .zip(versions.iter())
    {
        if let Err(e) = res {
            store_fails_info(
                Arc::clone(&conn),
                v.version_id,
                &v.name,
                &format!("Deal fails: {}", e),
            );
        } else {
            update_process_status(Arc::clone(&conn), v.version_id, "done");
        }
    }
}

fn deal_one_version(
    conn: Arc<Mutex<Client>>,
    version: &VersionInfo,
    home: &str,
    offline: bool,
) -> Result<(), Error> {
    let mut features = vec![];
    let mut to_dos = vec![];
    let mut edition = String::from("2015");

    let dir = if !offline {
        let data = File::open(&format!("{}/{}-{}.tgz", home, version.name, version.num))?;
        let mut archive = Archive::new(GzDecoder::new(data));

        archive.unpack(&format!("{}/{}-{}", home, version.name, version.num))?;
        WalkDir::new(&format!("{}/{}-{}", home, version.name, version.num))
    } else {
        WalkDir::new(&format!(
            "{}/{}/{}-{}",
            home, version.name, version.name, version.num
        ))
    };

    for entry in dir {
        let entry = entry?;

        if entry
            .path()
            .file_name()
            .unwrap()
            .eq_ignore_ascii_case("lib.rs")
        {
            to_dos.push(entry.path().to_owned());
        } else if entry
            .path()
            .file_name()
            .unwrap()
            .eq_ignore_ascii_case("Cargo.toml")
        {
            let mut file = File::open(entry.path())?;
            let mut buf = String::new();
            file.read_to_string(&mut buf).unwrap();

            let toml = buf.parse::<Value>()?;
            edition = toml
                .get("package")
                .map(|v| {
                    v.get("edition")
                        .map(|v| v.as_str().unwrap_or("2015"))
                        .unwrap_or("2015")
                })
                .unwrap_or("2015")
                .to_owned();
        }
    }

    for librs in &mut to_dos {
        let exec = Command::new(RUSTC)
            .arg("--edition")
            .arg(&edition)
            .arg("--nft-analysis")
            .arg(&librs)
            .output()?;

        if exec.status.success() {
            let out = String::from_utf8(exec.stdout).unwrap();

            lazy_static! {
                static ref RE: Regex = Regex::new(r#"\(\[(.*?)\], (.*?)\)"#).unwrap();
            }

            RE.captures_iter(&out)
                .map(|cap| {
                    if let (Some(cond), Some(feat)) = (cap.get(1), cap.get(2)) {
                        features.push((cond.as_str().to_string(), feat.as_str().to_string()));
                    }
                })
                .count();
        } else {
            let out = String::from_utf8(exec.stderr).unwrap();

            if out.contains("rustc resolve feature fails") {
                return Err(anyhow!(
                    "rustc analysis {} {} fails",
                    version.name,
                    version.num
                ));
            } else {
                return Err(anyhow!(
                    "rustc other fails, detail:{}, version: {} {}",
                    out.lines().nth(0).unwrap(),
                    version.name,
                    version.num
                ));
            }
        }
    }

    let mut query = String::new();

    if features.is_empty() {
        query.push_str(&format!(
            "INSERT INTO version_feature (id) VALUES('{}');",
            version.version_id
        ));
    } else {
        query.push_str("INSERT INTO version_feature VALUES");
        features
            .iter()
            .map(|(cond, feat)| {
                query.push_str(&format!(
                    "('{}', '{}', '{}'),",
                    version.version_id, cond, feat
                ));
            })
            .count();
        query.pop();
        query.push(';');
    }

    conn.lock().unwrap().query(&query, &[]).unwrap_or_default();

    Ok(())
}

fn prebuild_db_table(conn: Arc<Mutex<Client>>) {
    conn.lock()
        .unwrap()
        .query(
            r#"CREATE TABLE IF NOT EXISTS public.version_feature
            (
                id INT,
                conds VARCHAR(255),
                feature VARCHAR(40) DEFAULT 'no_feature_used'
            )"#,
            &[],
        )
        .unwrap();

    conn.lock()
        .unwrap()
        .query(
            r#"CREATE TABLE IF NOT EXISTS public.feature_errors
            (
                version_id INT,
                crate_name VARCHAR(40),
                info TEXT,
                time TIMESTAMP DEFAULT current_timestamp
            )"#,
            &[],
        )
        .unwrap();
    conn.lock().unwrap().query(r#"CREATE VIEW versions_with_name as (
            SELECT versions.*, crates.name FROM versions INNER JOIN crates ON versions.crate_id = crates.id
            )"#, &[]).unwrap_or_default();

    conn.lock()
        .unwrap()
        .query(
            r#"CREATE TABLE IF NOT EXISTS public.feature_process_status
            (
                version_id INT,
                status VARCHAR
            )"#,
            &[],
        )
        .unwrap();

    // Check if table is empty
    if conn
        .lock()
        .unwrap()
        .query("SELECT * FROM feature_process_status LIMIT 1", &[])
        .unwrap()
        .first()
        .is_none()
    {
        conn.lock()
            .unwrap()
            .query(
                r#"INSERT INTO feature_process_status (
                    SELECT id, 'undone' FROM versions
                )"#,
                &[],
            )
            .unwrap();
    } else {
        conn.lock()
            .unwrap()
            .query(
                r#"UPDATE feature_process_status SET status='fail' WHERE version_id IN (
                    SELECT version_id FROM feature_process_status WHERE status='processing'
                )"#,
                &[],
            )
            .unwrap();
    }
}

fn update_process_status(conn: Arc<Mutex<Client>>, version_id: i32, status: &str) {
    conn.lock()
        .unwrap()
        .query(
            &format!(
                "UPDATE feature_process_status SET status = '{}' WHERE version_id = '{}';",
                status, version_id
            ),
            &[],
        )
        .expect("Update process status fails");
}

#[allow(unused)]
fn get_versions_info(conn: Arc<Mutex<Client>>, version_ids: Vec<i32>) -> Vec<(i32, String)> {
    let query = format!(
        "SELECT id, version FROM versions WHERE id IN ({})",
        version_ids
            .iter()
            .map(|v| format!("{}", v))
            .collect::<Vec<String>>()
            .join(",")
    );
    let rows = conn.lock().unwrap().query(&query, &[]).unwrap();
    rows.iter().map(|row| (row.get(0), row.get(1))).collect()
}

fn store_fails_info(conn: Arc<Mutex<Client>>, version_id: i32, name: &str, info: &str) {
    warn!("fails: {} {}", version_id, info);
    conn.lock()
        .unwrap()
        .query(
            &format!(
                "INSERT INTO feature_errors VALUES('{}', '{}', '{}');",
                version_id, name, info.replace("'", "''")
            ),
            &[],
        )
        .expect(&format!("Fatal error, store info {} fails!", info));

    update_process_status(conn, version_id, "fail");
}

#[test]
fn test() {
    let data = File::open("vcpkg-0.2.15.tgz").unwrap();
    let libfile = "lib.rs";
    let mut edition = String::from("2015");
    let mut archive = Archive::new(GzDecoder::new(data));
    let mut features = vec![];
    let mut to_dos = vec![];

    for file in &mut archive.entries().unwrap() {
        let mut file = file.unwrap();
        let file_name = file
            .header()
            .path()
            .unwrap()
            .file_name()
            .expect("Fatal error, get file name fails")
            .to_owned();

        if file_name.eq_ignore_ascii_case("lib.rs") {
            let mut buf = String::new();
            file.read_to_string(&mut buf).unwrap();

            to_dos.push(buf)
        } else if file_name.eq_ignore_ascii_case("Cargo.toml") {
            let mut buf = String::new();
            file.read_to_string(&mut buf).unwrap();

            let toml = buf.parse::<Value>().unwrap();
            edition = toml
                .get("package")
                .map(|v| {
                    v.get("edition")
                        .map(|v| v.as_str().unwrap_or("2015"))
                        .unwrap_or("2015")
                })
                .unwrap_or("2015")
                .to_owned();
        }
    }

    for buf in to_dos {
        File::create(&libfile)
            .unwrap()
            .write_all(buf.as_bytes())
            .unwrap();

        let exec = Command::new(RUSTC)
            .arg("--edition")
            .arg(&edition)
            .arg("--nft-analysis")
            .arg(&libfile)
            .output()
            .unwrap();

        if exec.status.success() {
            let out = String::from_utf8(exec.stdout).unwrap();

            lazy_static! {
                static ref RE: Regex = Regex::new(r#"\(\[(.*?)\], (.*?)\)"#).unwrap();
            }

            RE.captures_iter(&out)
                .map(|cap| {
                    if let (Some(cond), Some(feat)) = (cap.get(1), cap.get(2)) {
                        features.push((cond.as_str().to_string(), feat.as_str().to_string()));
                    }
                })
                .count();
            println!("{:?}", features);
        } else {
            let out = String::from_utf8(exec.stderr).unwrap();

            if out.contains("rustc resolve feature fails") {
                println!("rustc analysis fails");
            } else {
                println!("rustc other fails, detail:{}", out.lines().nth(0).unwrap());
            }
        }
    }
}

// https://crates.io/api/v1/crates/$(crate)/$(version)/download
