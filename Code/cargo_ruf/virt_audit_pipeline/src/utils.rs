/*
    Sample DB Settins:

    CREATE TABLE indirect_impact AS (
    SELECT version_from as id, status
    FROM dep_version_feature
    INNER JOIN feature_status ON name=nightly_feature )

    CREATE TABLE virt_audit_process AS (
    SELECT DISTINCT(ver) as id, 'undone' as status
    FROM (
        SELECT *
        FROM indirect_impact TABLESAMPLE SYSTEM (10) -- 10% Random Sampling
    ) sampled
    WHERE status = 'removed' OR status = 'unknown'
    LIMIT 700)
*/

use std::{
    any::Any,
    env::current_dir,
    fs::File,
    panic,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use async_std::future::timeout;
use crossbeam::channel;
use log::{error, info};
use postgres::{Client, NoTls};

use ruf_audit_virtual::{audit, AuditError};

pub struct VersionInfo {
    pub version_id: i32,
    pub name: String,
    pub num: String,
}

pub fn run_audit_virt(workers: usize, status: &str) {
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
    let (tx, rx) = channel::bounded(workers);

    let mut handles = Vec::new();
    for i in 0..workers {
        let conn = Arc::clone(&conn);
        let rx = rx.clone();

        handles.push(thread::spawn(move || {
            let workspace = current_dir()
                .expect("Failed to get current directory")
                .join("virt_audit_jobs")
                .join(format!("job{i}"));
            let workspace_str = workspace.to_str().unwrap();

            if !workspace.exists() {
                let tmp_dir = workspace.join("src");
                std::fs::create_dir_all(&tmp_dir).unwrap();

                let tmp_file = tmp_dir.join("lib.rs");
                File::create(&tmp_file).unwrap();
            }

            while let Ok(versions) = rx.recv() {
                for version in versions {
                    let version = version as VersionInfo;

                    info!("[{}] Start auditing {}@{}", i, &version.name, &version.num);
                    let output = Arc::new(Mutex::new(Vec::new()));

                    let start_time = Instant::now();
                    match async_std::task::block_on(limited_audit(
                        &version.name,
                        &version.num,
                        workspace_str,
                        Arc::clone(&output),
                    )) {
                        Ok(Ok(Ok(rustv))) => {
                            let duration = start_time.elapsed();
                            let output = String::from_utf8(output.lock().unwrap().to_vec())
                                .expect("cannot convert output to string");
                            store_audit_results(
                                Arc::clone(&conn),
                                version.version_id,
                                "success",
                                Some(rustv as i32),
                                None,
                                Some(&output),
                                duration,
                            );
                            update_process_status(Arc::clone(&conn), version.version_id, "done");

                            info!("[{}] Done auditing: {}@{}", i, &version.name, &version.num);
                        }
                        Ok(Ok(Err(e))) => {
                            let duration = start_time.elapsed();
                            let (status, error) = match e {
                                AuditError::InnerError(error) => ("inner fail", error),
                                AuditError::FunctionError(_, _) => {
                                    ("fix fail", "all methods failed".to_string())
                                }
                            };
                            let output = String::from_utf8(output.lock().unwrap().to_vec())
                                .expect("cannot convert output to string");
                            store_audit_results(
                                Arc::clone(&conn),
                                version.version_id,
                                status,
                                None,
                                Some(&error),
                                Some(&output),
                                duration,
                            );
                            update_process_status(Arc::clone(&conn), version.version_id, status);
                        }
                        Ok(Err(e)) => {
                            let duration = start_time.elapsed();
                            let e = if let Some(s) = e.downcast_ref::<String>() {
                                s.as_str()
                            } else {
                                "Uncatched panic"
                            };

                            store_audit_results(
                                Arc::clone(&conn),
                                version.version_id,
                                "panic",
                                None,
                                Some(e),
                                None,
                                duration,
                            );

                            error!("[{}] audit panic: {:?}", i, e);
                            update_process_status(Arc::clone(&conn), version.version_id, "panic");
                        }
                        Err(_) => {
                            let duration = start_time.elapsed();
                            store_audit_results(
                                Arc::clone(&conn),
                                version.version_id,
                                "timeout",
                                None,
                                None,
                                None,
                                duration,
                            );
                            update_process_status(Arc::clone(&conn), version.version_id, "timeout");
                        }
                    }
                }
            }
        }));
    }

    // Send todo versions.
    loop {
        let rows = conn
            .lock()
            .unwrap()
            .query(
                r#"SELECT id, name, num FROM versions_with_name WHERE id in (
                    SELECT id FROM virt_audit_process WHERE status=$1 ORDER BY id asc LIMIT 20
                )"#,
                &[&status],
            )
            .unwrap();

        if rows.is_empty() {
            break;
        } else {
            conn.lock()
                .unwrap()
                .query(
                    r#"UPDATE virt_audit_process SET status='processing' WHERE id IN (
                        SELECT id FROM virt_audit_process WHERE status=$1 ORDER BY id asc LIMIT 20
                        )"#,
                    &[&status],
                )
                .unwrap();

            let versions: Vec<VersionInfo> = rows
                .iter()
                .map(|row| VersionInfo {
                    version_id: row.get(0),
                    name: row.get(1),
                    num: row.get(2),
                })
                .collect();

            tx.send(versions).unwrap();
        }
    }

    drop(tx);
    for handle in handles {
        // Unsolved problem
        if handle.join().is_err() {
            error!("!!!Thread Crash!!!")
        }
    }

    info!(r#"\\\ !Auditing Done! ///"#);
}

fn prebuild(conn: Arc<Mutex<Client>>) {
    conn.lock()
        .unwrap()
        .query(
            &format!(
                r#"CREATE TABLE IF NOT EXISTS virt_audit_results(
                    version_id INT PRIMARY KEY,
                    result VARCHAR,
                    rustv INT,
                    error  VARCHAR,
                    output TEXT,
                    time_duration BIGINT
                )"#
            ),
            &[],
        )
        .unwrap();

    conn.lock()
        .unwrap()
        .query(
            &format!("UPDATE virt_audit_process SET status='undone' WHERE status='processing'"),
            &[],
        )
        .unwrap();
}

fn store_audit_results(
    conn: Arc<Mutex<Client>>,
    version_id: i32,
    result: &str,
    rustv: Option<i32>,
    error: Option<&str>,
    output: Option<&str>,
    time_duration: std::time::Duration,
) {
    let time_duration_secs = time_duration.as_secs() as i64;
    conn.lock()
    .unwrap()
    .query(
        "INSERT INTO virt_audit_results(version_id, result, rustv, error, output, time_duration) VALUES($1, $2, $3, $4, $5, $6)
        ON CONFLICT (version_id) DO UPDATE SET result = EXCLUDED.result, rustv = EXCLUDED.rustv, error = EXCLUDED.error, output = EXCLUDED.output, time_duration = EXCLUDED.time_duration",
        &[&version_id, &result, &rustv, &error, &output, &time_duration_secs],
    ).unwrap();
}

fn update_process_status(conn: Arc<Mutex<Client>>, version_id: i32, status: &str) {
    conn.lock()
        .unwrap()
        .query(
            &format!(r#"UPDATE virt_audit_process SET status=$1 WHERE id=$2"#),
            &[&status, &version_id],
        )
        .expect("cannot update process status");
}

async fn limited_audit(
    name: &str,
    ver: &str,
    workspace: &str,
    output: Arc<Mutex<Vec<u8>>>,
) -> Result<Result<Result<u32, AuditError>, Box<dyn Any + Send>>, ()> {
    let result = timeout(Duration::from_secs(5 * 60), async {
        panic::catch_unwind(|| audit(name, ver, workspace, &mut *output.lock().unwrap()))
    })
    .await;

    match result {
        Ok(Ok(res)) => Ok(Ok(res)),
        Ok(Err(e)) => Ok(Err(e)),
        Err(_) => Err(()), // Time out
    }
}
