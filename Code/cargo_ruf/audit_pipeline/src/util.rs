use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

use crossbeam::channel;
use lazy_static::lazy_static;
use log::{error, info, warn};
use postgres::{Client, NoTls};
use regex::Regex;

// Suffix of DB, used for test and other purposes.
// It is empty by default. If it is not, it is not used for general purposes.
const DB_SUFFIX: &str = "";
const RUF_AUDIT: &str = "../ruf_audit/target/debug/ruf_audit";
const ON_PROCESS: &str = "../../crate_downloader/on_process/";
const THREAD_DATA_SIZE: u32 = 20;

lazy_static! {
    static ref COLOR_CODES: Regex = Regex::new("\x1b\\[[^m]*m").unwrap();
}

pub struct VersionInfo {
    pub version_id: i32,
    pub crate_id: i32,
    pub name: String,
    pub num: String,
}

pub fn run_audit(workers: usize, status: &str) {
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
    prebuild(Arc::clone(&conn));

    println!("Creating Channel");
    // Create channel
    let (tx, rx) = channel::bounded(workers);

    let mut handles = Vec::new();
    for i in 0..workers {
        let conn = conn.clone();
        let rx = rx.clone();

        handles.push(thread::spawn(move || {
            let ruf_audit_path = PathBuf::from(RUF_AUDIT).canonicalize().expect("cannot find ruf_audit");
            let cargo_home = PathBuf::from(format!("./cargo_usage/home{i}")).canonicalize().expect("cannot find cargo home");

            while let Ok(versions) = rx.recv() {
                for v in versions {
                    let v = v as VersionInfo;
                    let mut audit = Command::new(&ruf_audit_path);
                    let work_dir = PathBuf::from(format!(
                        "{ON_PROCESS}/{name}/{name}-{num}",
                        name = v.name,
                        num = v.num
                    ))
                    .canonicalize();
                    let work_dir = match work_dir {
                        Ok(work_dir) => work_dir,
                        Err(e) => {
                            warn!(
                                "Thread {i}: transform path to {name}-{num} fails, due to error: {e}",
                                name = v.name, num = v.num
                            );
                            store_audit_results(
                                Arc::clone(&conn),
                                v.version_id,
                                -1,
                                &e.to_string(),
                            );
                            update_process_status(Arc::clone(&conn), v.version_id, "fail");
                            continue;
                        }
                    };


                    audit.current_dir(&work_dir);
                    audit.args(["--quick-fix", "--", "--all-features", "--all-targets"]);
                    audit.env("CARGO_HOME", &cargo_home);

                    info!("Thread {i}: audit version {version} start", version = v.version_id);

                    let output = match audit.output() {
                        Ok(output) => output,
                        Err(e) => {
                            warn!(
                                "Thread {i}: audit version {version} fails, due to error: {e}",
                                version = v.version_id
                            );
                            store_audit_results(
                                Arc::clone(&conn),
                                v.version_id,
                                -1,
                                &e.to_string(),
                            );
                            update_process_status(Arc::clone(&conn), v.version_id, "fail");
                            continue;
                        }
                    };

                    info!("Thread {i}: audit version {version} end", version = v.version_id);

                    let msg = String::from_utf8_lossy(&output.stdout);
                    let msg = COLOR_CODES.replace_all(&msg, "");

                    if output.status.success() {
                        store_audit_results(
                            Arc::clone(&conn),
                            v.version_id,
                            output.status.code().unwrap_or(0),
                            &msg,
                        );
                        update_process_status(Arc::clone(&conn), v.version_id, "done");
                    } else {
                        store_audit_results(
                            Arc::clone(&conn),
                            v.version_id,
                            output.status.code().unwrap_or(-1),
                            &msg,
                        );
                        update_process_status(Arc::clone(&conn), v.version_id, "fail");
                    }

                    // do cleaning
                    let mut cargo = Command::new("cargo");
                    cargo.arg("clean");
                    cargo.current_dir(&work_dir);
                    if matches!(cargo.output().map(|output| output.status.success()), Err(_) | Ok(false)) {
                        warn!("Thread {i}: cleaning up for {version} failed", version = v.version_id);
                    }
                } // for ends
            } // while ends
        }));
    }

    // send versions
    loop {
        let conn = Arc::clone(&conn);
        let query = format!(
            r#"SELECT id,crate_id,name,num FROM versions_with_name WHERE id in (
                SELECT version_id FROM ruf_audit_process_status{DB_SUFFIX} WHERE status='{}' ORDER BY version_id asc LIMIT {}
                )"#,
            status, THREAD_DATA_SIZE
        );

        let rows = conn.lock().unwrap().query(&query, &[]).unwrap();
        if rows.is_empty() {
            break;
        } else {
            let query = format!(
                r#"UPDATE ruf_audit_process_status{DB_SUFFIX} SET status='processing' WHERE version_id IN (
                    SELECT version_id FROM ruf_audit_process_status{DB_SUFFIX} WHERE status='{}' ORDER BY version_id asc LIMIT {}
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

fn prebuild(conn: Arc<Mutex<Client>>) {
    conn.lock()
        .unwrap()
        .query(
            &format!(
                r#"CREATE TABLE IF NOT EXISTS ruf_audit_results{DB_SUFFIX}(
                    version_id INT PRIMARY KEY,
                    exit_code INT,
                    msg VARCHAR
                )"#
            ),
            &[],
        )
        .unwrap_or_default();

    conn.lock()
        .unwrap()
        .query(
            &format!(
                r#"CREATE TABLE IF NOT EXISTS ruf_audit_process_status{DB_SUFFIX}
            (
                version_id INT,
                status VARCHAR
            )"#
            ),
            &[],
        )
        .unwrap();

    if conn
        .lock()
        .unwrap()
        .query(
            &format!("SELECT * FROM ruf_audit_process_status{DB_SUFFIX} LIMIT 1"),
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
                    INSERT INTO ruf_audit_process_status{DB_SUFFIX} 
                    SELECT DISTINCT ver, 'undone' FROM tmp_ruf_impact WHERE status = 'removed' or status = 'unknown'"
                ),
                &[],
            )
            .unwrap();
    } else {
        conn.lock()
            .unwrap()
            .query(
            &format!("UPDATE ruf_audit_process_status{DB_SUFFIX} SET status='undone' WHERE status='processing'"),
            &[]
            ).unwrap();
    }
}

fn store_audit_results(conn: Arc<Mutex<Client>>, version_id: i32, exit_code: i32, msg: &str) {
    conn.lock()
        .unwrap()
        .query(
            &format!(
                r#"INSERT INTO ruf_audit_results{DB_SUFFIX} (version_id, exit_code, msg)
                VALUES ($1, $2, $3)
                ON CONFLICT (version_id)
                DO UPDATE SET exit_code = EXCLUDED.exit_code, msg = EXCLUDED.msg"#
            ),
            &[&version_id, &exit_code, &msg],
        )
        .expect("cannot store audit results");
}

fn update_process_status(conn: Arc<Mutex<Client>>, version_id: i32, status: &str) {
    // warn!("update status");
    conn.lock()
        .unwrap()
        .query(
            &format!(
                r#"UPDATE ruf_audit_process_status{DB_SUFFIX} SET status=$1 WHERE version_id=$2"#
            ),
            &[&status, &version_id],
        )
        .expect("cannot update process status");
}
