use std::collections::HashSet;
use std::fs::{create_dir, remove_dir_all, File};
use std::io::Read;
use std::panic::{self, catch_unwind};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{anyhow, Context, Result};
use crossbeam::channel::{self};
use downloader::{Download, Downloader};
use flate2::read::GzDecoder;
use lazy_static::lazy_static;
use log::{error, info, warn};
use pbr::MultiBar;
use postgres::{Client, NoTls};
use regex::Regex;
use tar::Archive;

// https://crates.io/api/v1/crates/$(crate)/$(version)/download

#[allow(unused)]
pub fn run(workers: usize, todo_status: &str) {
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
            r#"CREATE TABLE IF NOT EXISTS public.version_feature
            (
                id INT,
                feature VARCHAR(40) DEFAULT 'no_feature_used',
                UNIQUE(id, feature)
            )"#,
            &[],
        )
        .unwrap();

    conn.lock()
        .unwrap()
        .query(
            r#"CREATE TABLE IF NOT EXISTS public.fails_info
            (
                crate_id INT,
                crate_name VARCHAR(40),
                info VARCHAR(40),
                time TIMESTAMP DEFAULT current_timestamp
            )"#,
            &[],
        )
        .unwrap();

    let todo_count: i64 = conn
        .lock()
        .unwrap()
        .query(
            &format!(
                "SELECT COUNT(crate_id) FROM process_status WHERE status = '{}'",
                todo_status
            ),
            &[],
        )
        .unwrap()
        .first()
        .unwrap()
        .get(0);

    create_dir(Path::new(&format!("on_process"))).unwrap_or_default();

    let mb = Arc::new(MultiBar::new());
    let mut mpb = mb.create_bar(todo_count as u64);
    mpb.format("╢▌▌░╟");
    mpb.set(0);

    let (tx, rx) = channel::bounded(2 * workers);

    let mut handles = vec![];
    for i in 0..workers {
        let rx = rx.clone();
        let conn = Arc::clone(&conn);
        let mb = Arc::clone(&mb);

        // Start Fetching
        handles.push(thread::spawn(move || {
            let old_hook = panic::take_hook();
            panic::set_hook({
                Box::new(move |info| {
                    error!("Thread {}: panic, {}", i, info);
                })
            });

            catch_unwind(|| {
                let mut pb = mb.create_bar(2);
                let mut downloader = Downloader::builder()
                    .download_folder(Path::new("./on_process"))
                    .parallel_requests(1)
                    .build()
                    .expect("Fatal Error, build downloader fails!");

                while let Ok((id, vers)) = rx.recv() {
                    let name = get_name_by_crate_id(Arc::clone(&conn), id)
                        .expect("Fatal Error, get crates name fails!");

                    pb.set(0);
                    pb.message(&name);

                    if let Err(e) = fetch_crate(&mut downloader, &name, &vers) {
                        warn!("Thread {}: Fetch fails: crate {} {}, {}", i, id, name, e);
                        store_fails_info(Arc::clone(&conn), id, &name, &e.to_string())
                    } else {
                        pb.inc();
                        if let Err(e) = deal_crate(Arc::clone(&conn), &name, id, &vers) {
                            warn!("Thread {}: Deal fails: crate {} {}, {}", i, id, name, e);
                            store_fails_info(Arc::clone(&conn), id, &name, &e.to_string())
                        } else {
                            pb.inc();
                            info!("Thread {}: Done crates - {}", i, id);
                        }
                    }
                    remove_dir_all(&format!("on_process/{}", name)).unwrap_or_default();
                }

                pb.finish();
            })
            .unwrap_or_default();
            panic::set_hook(old_hook);
        }));
    }

    handles.push(thread::spawn(move || mb.listen()));

    loop {
        let conn = Arc::clone(&conn);
        let query = format!(
            "SELECT crate_id FROM process_status WHERE status='{}' ORDER BY crate_id asc LIMIT 250",
            todo_status
        );

        let rows = conn.lock().unwrap().query(&query, &[]).unwrap();
        if rows.is_empty() {
            break;
        } else {
            let crate_ids: Vec<i32> = rows.iter().map(|crate_id| crate_id.get(0)).collect();
            for crate_id in crate_ids {
                let vers = get_versions_by_crate_id(Arc::clone(&conn), crate_id);
                tx.send((crate_id, vers)).expect("Fatal error, send fails");
                mpb.inc();
            }
        }
    }

    std::mem::drop(tx);

    mpb.finish();

    for handle in handles {
        // Unsolved problem
        if handle.join().is_err() {
            error!("!!!Thread Crash!!!")
        }
    }

    println!(r#"\\\ Done! ///"#)
}

fn fetch_crate(
    downloader: &mut Downloader,
    name: &str,
    versions: &Vec<(i32, String)>,
) -> Result<()> {
    let mut dls = vec![];

    create_dir(Path::new(&format!("on_process/{}", name))).unwrap_or_default();

    for (_, ver) in versions {
        dls.push(
            Download::new(&format!(
                "https://crates.io/api/v1/crates/{}/{}/download",
                name, ver
            ))
            .file_name(Path::new(&format!("{}/{}.tgz", name, ver))),
        );
    }

    let res = downloader.download(&dls)?;

    if res.iter().any(|res| res.is_err()) {
        return Err(anyhow!("Download error."));
    }

    return Ok(());
}

fn deal_crate(
    conn: Arc<Mutex<Client>>,
    name: &str,
    crate_id: i32,
    versions: &Vec<(i32, String)>,
) -> Result<()> {
    let mut query = String::new();

    for (version_id, ver) in versions {
        query.clear();

        let data = File::open(&format!("on_process/{}/{}.tgz", name, ver))?;
        let mut archive = Archive::new(GzDecoder::new(data));
        let mut features = HashSet::new();

        for file in archive.entries()? {
            let mut file = file?;
            if file
                .header()
                .path()?
                .file_name()
                .unwrap()
                .eq_ignore_ascii_case("lib.rs")
            {
                let mut buf = String::new();
                file.read_to_string(&mut buf)?;
                lazy_static! {
                    static ref RE: Regex = Regex::new(r"/\*[\s\S]*?\*/|//.*|#!\[feature\((.*?)\)\]").unwrap();
                }
                RE.captures_iter(&buf)
                    .map(|cap| {
                        if let Some(cap) = cap.get(1) {
                            features.extend(
                                cap.as_str()
                                    .split(',')
                                    .map(|s| s.trim().to_string())
                                    .collect::<Vec<String>>(),
                            );
                        }
                    })
                    .count();
            }
        }

        if features.is_empty() {
            query.push_str(&format!(
                "INSERT INTO version_feature (id) VALUES('{}');",
                version_id
            ));
        } else {
            query.push_str("INSERT INTO version_feature VALUES");
            features
                .iter()
                .map(|feature| {
                    query.push_str(&format!("('{}', '{}'),", version_id, feature));
                })
                .count();
            query.pop();
            query.push(';');
        }

        conn.lock().unwrap().query(&query, &[]).unwrap_or_default();
    }

    conn.lock().unwrap().query(
        &format!(
            "UPDATE process_status SET status = 'done' WHERE crate_id = '{}';",
            crate_id
        ),
        &[],
    )?;

    Ok(())
}

fn get_name_by_crate_id(conn: Arc<Mutex<Client>>, crate_id: i32) -> Result<String> {
    let query = format!("SELECT name FROM crates WHERE id = {} LIMIT 1", crate_id);
    let row = conn.lock().unwrap().query(&query, &[]).unwrap();
    Ok(row
        .first()
        .with_context(|| format!("Get name by crate id fails, crate id: {}", crate_id))?
        .get(0))
}

fn get_versions_by_crate_id(conn: Arc<Mutex<Client>>, crate_id: i32) -> Vec<(i32, String)> {
    let query = format!(
        "SELECT id,num FROM versions WHERE crate_id = '{}'",
        crate_id
    );

    let row = conn.lock().unwrap().query(&query, &[]).unwrap();
    row.iter().map(|ver| (ver.get(0), ver.get(1))).collect()
}

fn store_fails_info(conn: Arc<Mutex<Client>>, crate_id: i32, name: &str, info: &str) {
    conn.lock()
        .unwrap()
        .query(
            &format!(
                "INSERT INTO fails_info VALUES('{}', '{}', '{}');",
                crate_id, name, info
            ),
            &[],
        )
        .expect("Fatal error, store info fails!");
    conn.lock()
        .unwrap()
        .query(
            &format!(
                "UPDATE process_status SET status = 'fails' WHERE crate_id = '{}';",
                crate_id
            ),
            &[],
        )
        .expect("Fatal error, store info fails!");
}

#[allow(unused)]
/// Processing crates, even if it could fails
/// 收尾处理
pub fn run_imcomplete(todo_status: &str) {
    let conn = Arc::new(Mutex::new(
        Client::connect(
            "host=localhost dbname=crates user=postgres password=postgres",
            NoTls,
        )
        .unwrap(),
    ));

    create_dir(Path::new(&format!("on_process"))).unwrap_or_default();

    let mut downloader = Downloader::builder()
        .download_folder(Path::new("./on_process"))
        .build()
        .expect("Fatal Error, build downloader fails!");

    let query = format!(
        "SELECT crate_id FROM process_status WHERE status='{}' ORDER BY crate_id asc",
        todo_status
    );

    let rows = conn.lock().unwrap().query(&query, &[]).unwrap();
    let crate_ids: Vec<i32> = rows.iter().map(|crate_id| crate_id.get(0)).collect();

    for crate_id in crate_ids {
        let name = get_name_by_crate_id(Arc::clone(&conn), crate_id).unwrap();
        let vers = get_versions_by_crate_id(Arc::clone(&conn), crate_id);
        if let Err(e) = fetch_crate(&mut downloader, &name, &vers) {
            warn!("Imcomplete fetch: crate {} {}, {}", crate_id, name, e);
        }

        if deal_crate_imcomplete(Arc::clone(&conn), &name, crate_id, &vers) {
            info!("Complete deal: crates {} {}", crate_id, name);
        } else {
            warn!("Imcomplete deal: crate {} {}", crate_id, name);
        }
    }
}

fn deal_crate_imcomplete(
    conn: Arc<Mutex<Client>>,
    name: &str,
    crate_id: i32,
    versions: &Vec<(i32, String)>,
) -> bool {
    let res: Vec<bool> = versions
        .iter()
        .map(|(version_id, ver)| {
            if deal_one_imcomplete(Arc::clone(&conn), name, version_id, ver).is_ok() {
                info!("Done part {} {}", name, ver);
                true
            } else {
                info!("Fail part {} {}", name, ver);
                false
            }
        })
        .collect();

    if res.iter().all(|&res| res) {
        conn.lock()
            .unwrap()
            .query(
                &format!(
                    "UPDATE process_status SET status = 'done' WHERE crate_id = '{}';",
                    crate_id
                ),
                &[],
            )
            .unwrap();
        true
    } else {
        conn.lock()
            .unwrap()
            .query(
                &format!(
                    "UPDATE process_status SET status = 'imcomplete' WHERE crate_id = '{}';",
                    crate_id
                ),
                &[],
            )
            .unwrap();
        false
    }
}

fn deal_one_imcomplete(
    conn: Arc<Mutex<Client>>,
    name: &str,
    version_id: &i32,
    ver: &str,
) -> Result<()> {
    let mut query = String::new();

    let data = File::open(&format!("on_process/{}/{}.tgz", name, ver))?;
    let mut archive = Archive::new(GzDecoder::new(data));
    let mut features = HashSet::new();

    for file in archive.entries()? {
        let mut file = file?;
        if file
            .header()
            .path()?
            .file_name()
            .unwrap()
            .eq_ignore_ascii_case("lib.rs")
        {
            let mut buf = String::new();
            file.read_to_string(&mut buf)?;
            lazy_static! {
                static ref RE: Regex = Regex::new(r"//.*|#!\[feature\((.*?)\)\]").unwrap();
            }
            RE.captures_iter(&buf)
                .map(|cap| {
                    if let Some(cap) = cap.get(1) {
                        features.extend(
                            cap.as_str()
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .collect::<Vec<String>>(),
                        );
                    }
                })
                .count();
        }
    }

    if features.is_empty() {
        query.push_str(&format!(
            "INSERT INTO version_feature (id) VALUES('{}');",
            version_id
        ));
    } else {
        query.push_str("INSERT INTO version_feature VALUES");
        features
            .iter()
            .map(|feature| {
                query.push_str(&format!("('{}', '{}'),", version_id, feature));
            })
            .count();
        query.pop();
        query.push(';');
    }

    conn.lock().unwrap().query(&query, &[]).unwrap_or_default();
    Ok(())
}
