use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::Result;
use crossbeam::channel::{self};
use log::{error, info, warn};
use postgres::{Client, NoTls};

pub fn run(workers: usize) {
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
            r#"CREATE TABLE IF NOT EXISTS public.dep_feature
            (
                source_version INT,
                version_from INT,
                version_to INT,
                distance INT,
                UNIQUE(source_version, version_from, version_to)
            )"#,
            &[],
        )
        .unwrap();

    let (tx, rx) = channel::bounded(workers * 2);

    let mut handles = vec![];
    for i in 0..workers {
        let conn = conn.clone();
        let rx = rx.clone();

        handles.push(thread::spawn(move || {
            while let Ok(sid) = rx.recv() {
                if let Err(e) = run_one_version(Arc::clone(&conn), sid) {
                    warn!("Thread {}: run {} fails, {}", i, sid, e);
                } else {
                    info!("Thread {}: finish {}", i, sid);
                }
            }
        }));
    }

    loop {
        let conn = Arc::clone(&conn);
        let query = format!(
            "SELECT sid FROM process_status_propagation WHERE status='undone' ORDER BY sid asc LIMIT 250");

        let rows = conn.lock().unwrap().query(&query, &[]).unwrap();
        if rows.is_empty() {
            break;
        } else {
            let tasks: Vec<i32> = rows.iter().map(|row| row.get(0)).collect();
            for task in tasks {
                tx.send(task).expect("Fatal error, send fails");
            }
        }
    }

    std::mem::drop(tx);


    for handle in handles {
        if handle.join().is_err() {
            error!("!!!Thread Crash!!!")
        }
    }

    println!(r#"\\\ Done! ///"#)
}

fn run_one_version(conn: Arc<Mutex<Client>>, sid: i32) -> Result<()> {
    let mut tasks = VecDeque::new();
    let mut dones = HashSet::new();
    tasks.push_back(sid);

    conn.lock().unwrap().query(
        &format!(
            "UPDATE process_status_propagation SET status = 'processing' WHERE sid = '{}';",
            sid
        ),
        &[],
    )?;


    while let Some(vid) = tasks.pop_front() {
        if dones.contains(&vid) {
            continue;
        }

        let vids: Vec<i32> = conn
            .lock()
            .unwrap()
            .query(
                &format!(
                    "SELECT DISTINCT version_from FROM dep_version WHERE version_to = {}",
                    vid
                ),
                &[],
            )?
            .into_iter()
            .map(|v| v.get(0))
            .collect();

        conn.lock()
            .unwrap()
            .query(
                &format!(
                    "INSERT INTO dep_feature(
            SELECT {} as sid,version_to,version_from,min(dep_level)
            FROM dep_version WHERE version_to = {} GROUP BY version_to,version_from)",
                    sid, vid
                ),
                &[],
            )
            .unwrap_or_default();

        dones.insert(vid);
        tasks.extend(vids)
    }

    conn.lock().unwrap().query(
        &format!(
            "UPDATE process_status_propagation SET status = 'done' WHERE sid = '{}';",
            sid
        ),
        &[],
    )?;

    Ok(())
}
