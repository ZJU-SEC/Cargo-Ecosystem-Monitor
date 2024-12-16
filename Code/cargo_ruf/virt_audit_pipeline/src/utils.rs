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
    env::current_dir,
    fs::File,
    panic,
    sync::{Arc, Mutex},
    thread,
};

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

                    match panic::catch_unwind(|| {
                        audit(
                            &version.name,
                            &version.num,
                            workspace_str,
                            &mut *output.lock().unwrap(),
                        )
                    }) {
                        Ok(audit_result) => {
                            info!("[{}] Done auditing: {}@{}", i, &version.name, &version.num);
                            let output = String::from_utf8(output.lock().unwrap().to_vec())
                                .expect("cannot convert output to string");
                            match audit_result {
                                Ok(_) => {
                                    store_audit_results(
                                        Arc::clone(&conn),
                                        version.version_id,
                                        "success",
                                        None,
                                        None,
                                        Some(&output),
                                    );
                                    update_process_status(
                                        Arc::clone(&conn),
                                        version.version_id,
                                        "done",
                                    );
                                }
                                Err(e) => {
                                    let (status, issue_dep, error) = match e {
                                        AuditError::InnerError(error) => {
                                            ("inner fail", None, error)
                                        }
                                        AuditError::FunctionError(error, issue_dep) => {
                                            ("fix fail", issue_dep, error)
                                        }
                                    };
                                    store_audit_results(
                                        Arc::clone(&conn),
                                        version.version_id,
                                        status,
                                        issue_dep.as_ref().map(|s| s.as_str()),
                                        Some(&error),
                                        Some(&output),
                                    );
                                    update_process_status(
                                        Arc::clone(&conn),
                                        version.version_id,
                                        status,
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            let e = if let Some(s) = e.downcast_ref::<String>() {
                                s.as_str()
                            } else {
                                "Uncatched panic"
                            };

                            error!("[{}] audit panic: {:?}", i, e);
                            store_audit_results(
                                Arc::clone(&conn),
                                version.version_id,
                                "panic",
                                None,
                                Some(format!("{:?}", e).as_str()),
                                None,
                            );
                            update_process_status(Arc::clone(&conn), version.version_id, "panic");
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
                    error  VARCHAR,
                    issue_dep VARCHAR,
                    output TEXT
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
    issue_dep: Option<&str>,
    error: Option<&str>,
    output: Option<&str>,
) {
    conn.lock()
    .unwrap()
    .query(
        "INSERT INTO virt_audit_results(version_id, result, error, issue_dep, output) VALUES($1, $2, $3, $4, $5)
        ON CONFLICT (version_id) DO UPDATE SET result = EXCLUDED.result, error = EXCLUDED.error, issue_dep = EXCLUDED.issue_dep, output = EXCLUDED.output",
        &[&version_id, &result, &error, &issue_dep, &output],
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
