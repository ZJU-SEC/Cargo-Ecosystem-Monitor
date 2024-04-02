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
const CARGO_USAGE: &str = "../cargo_usage";
const ON_PROCESS: &str = "../../crate_downloader/on_process/";
const THREAD_DATA_SIZE: u32 = 20;

lazy_static! {
    static ref COLOR_CODES: Regex = Regex::new("\x1b\\[[^m]*m").unwrap();
    static ref TEST_RESULT: Regex =
        Regex::new(r"===\((true|false),(true|false),(true|false),(true|false)\)===").unwrap();
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
            let ruf_audit_path_str = ruf_audit_path.to_str().unwrap();
            let cargo_home = PathBuf::from(format!("{CARGO_USAGE}/home{i}")).canonicalize().expect("cannot find cargo home");

            while let Ok(versions) = rx.recv() {
                for v in versions {
                    let v = v as VersionInfo;
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
                                (false, false, false, false),
                                &e.to_string(),
                            );
                            update_process_status(Arc::clone(&conn), v.version_id, "error1");
                            continue;
                        }
                    };

                    // pre clean
                    let mut clean = Command::new("sh");
                    clean.args(["-c", "cargo clean && rm -f Cargo.lock"]);
                    clean.current_dir(&work_dir);
                    clean.env("CARGO_HOME", &cargo_home);
                    if matches!(clean.output().map(|output| output.status.success()), Err(_) | Ok(false)) {
                        warn!("Thread {i}: pre cleaning up for {version} failed", version = v.version_id);
                    }

                    let mut audit = Command::new("systemd-run");
                    audit.args([
                        "--scope",
                        "-p",
                        "MemoryMax=4G",
                        "--user",
                        ruf_audit_path_str,
                        "--test",
                        "--",
                        "--all-features",
                        "--all-targets",
                    ]);

                    // let mut audit = Command::new(&ruf_audit_path);
                    // audit.args(["--test", "--", "--all-features", "--all-targets"]);
                    audit.current_dir(&work_dir);
                    audit.env("CARGO_HOME", &cargo_home);
                    audit.env_remove("RUSTUP_TOOLCHAIN");

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
                                (false, false, false, false),
                                &e.to_string(),
                            );
                            update_process_status(Arc::clone(&conn), v.version_id, "error2");
                            continue;
                        }
                    };

                    info!("Thread {i}: audit version {version} end", version = v.version_id);

                    let msg = String::from_utf8_lossy(&output.stdout);
                    let msg = COLOR_CODES.replace_all(&msg, "");

                    let exit_code = output.status.code().unwrap_or(3);
                    if exit_code == 0 || exit_code == 2 {
                        let caps = TEST_RESULT.captures(&msg).unwrap();
                        let results = (
                            caps.get(1).unwrap().as_str() == "true",
                            caps.get(2).unwrap().as_str() == "true",
                            caps.get(3).unwrap().as_str() == "true",
                            caps.get(4).unwrap().as_str() == "true",
                        );

                        store_audit_results(Arc::clone(&conn), v.version_id, exit_code, results, &msg);
                        update_process_status(Arc::clone(&conn), v.version_id, "done");
                    } else {
                        // normally the exit code should be 1 if failed
                        store_audit_results(Arc::clone(&conn), v.version_id, exit_code, (false, false, false, false), &msg);
                        update_process_status(Arc::clone(&conn), v.version_id, "fail");
                    }

                    // do cleaning
                    clean.args(["-c", "cargo clean && rm -f Cargo.lock"]);
                    clean.current_dir(&work_dir);
                    clean.env("CARGO_HOME", &cargo_home);
                    if matches!(clean.output().map(|output| output.status.success()), Err(_) | Ok(false)) {
                        warn!("Thread {i}: pre cleaning up for {version} failed", version = v.version_id);
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
                    Test1 bool,
                    Test2 bool,
                    Test3 bool,
                    Test4 bool,
                    msg VARCHAR
                )"#
            ),
            &[],
        )
        .unwrap();

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
                    SELECT DISTINCT id, 'undone' FROM tmp_ruf_impact WHERE status = 'removed' or status = 'unknown'"
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

fn store_audit_results(
    conn: Arc<Mutex<Client>>,
    version_id: i32,
    exit_code: i32,
    results: (bool, bool, bool, bool),
    msg: &str,
) {
    conn.lock()
        .unwrap()
        .query(
            &format!(
                r#"INSERT INTO ruf_audit_results{DB_SUFFIX} (version_id, exit_code, Test1, Test2, Test3, Test4, msg)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT (version_id)
                DO UPDATE SET exit_code = EXCLUDED.exit_code, Test1 = EXCLUDED.Test1, Test2 = EXCLUDED.Test2, Test3 = EXCLUDED.Test3, Test4 = EXCLUDED.Test4, msg = EXCLUDED.msg"#
            ),
            &[
                &version_id,
                &exit_code,
                &results.0,
                &results.1,
                &results.2,
                &results.3,
                &msg,
            ],
        ).expect("cannot store audit results");
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
