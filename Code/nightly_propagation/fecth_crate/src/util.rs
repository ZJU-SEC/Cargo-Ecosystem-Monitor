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
use log::{error, info, warn};
use pbr::ProgressBar;
use postgres::{Client, NoTls};
use regex::Regex;
use tar::Archive;

// https://crates.io/api/v1/crates/$(crate)/$(version)/download

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
            r#"CREATE TABLE IF NOT EXISTS public.version_feature
            (
                id INT,
                feature VARCHAR(40)
            )"#,
            &[],
        )
        .unwrap();

    let all_count: i64 = conn
        .lock()
        .unwrap()
        .query("SELECT COUNT(id) FROM crates", &[])
        .unwrap()
        .first()
        .unwrap()
        .get(0);

    let undone_count: i64 = conn
        .lock()
        .unwrap()
        .query(
            "SELECT COUNT(crate_id) FROM process_status WHERE status = 'undone'",
            &[],
        )
        .unwrap()
        .first()
        .unwrap()
        .get(0);

    create_dir(Path::new(&format!("on_process"))).unwrap_or_default();

    let mut mpb = ProgressBar::new(all_count as u64);
    mpb.format("╢▌▌░╟");
    mpb.set((all_count - undone_count) as u64);

    let (tx, rx) = channel::bounded(workers);

    let mut handles = vec![];
    for i in 0..workers {
        let rx = rx.clone();
        let conn = Arc::clone(&conn);

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
                    .download_folder(Path::new("./on_process"))
                    .build()
                    .expect("Fatal Error, build downloader fails!");

                while let Ok((id, vers)) = rx.recv() {
                    let name = get_name_by_crate_id(Arc::clone(&conn), id)
                        .expect("Fatal Error, get crates name fails!");

                    if let Err(e) = fetch_crate(&mut downloader, &name, &vers) {
                        warn!("Thread {}: Fetch fails: crate {} {}, {}", i, id, name, e);
                    } else {
                        if let Err(e) = deal_crate(Arc::clone(&conn), &name, id, &vers) {
                            warn!("Thread {}: Deal fails: crate {} {}, {}", i, id, name, e);
                        } else {
                            info!("Thread {}: Done crates - {}", i, id);
                        }
                    }
                }
            })
            .unwrap_or_default();
            panic::set_hook(old_hook);
        }));
    }

    loop {
        let conn = Arc::clone(&conn);
        let query = format!("SELECT crate_id FROM process_status WHERE status='undone' LIMIT 250",);

        let rows = conn.lock().unwrap().query(&query, &[]).unwrap();
        if rows.is_empty() {
            break;
        } else {
            let crate_ids: Vec<i32> = rows.iter().map(|crate_id| crate_id.get(0)).collect();
            for crate_id in crate_ids {
                let vers = get_versions_by_crate_id(Arc::clone(&conn), crate_id);
                tx.send((crate_id, vers)).unwrap();
                mpb.inc();
            }
            break;
        }
    }

    std::mem::drop(tx);

    for handle in handles {
        // Unsolved problem
        if handle.join().is_err() {
            error!("!!!Thread Crash!!!")
        }
    }
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
        return Err(anyhow!("Download error, {:?}", res));
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
        let mut features = vec![];

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
                let re = Regex::new(r"#!\[feature\((.*)\)\]")?;
                re.captures_iter(&buf)
                    .map(|cap| {
                        features.extend(
                            cap.get(1)
                                .unwrap()
                                .as_str()
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .collect::<Vec<String>>(),
                        );
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

        conn.lock().unwrap().query(&query, &[])?;
    }

    conn.lock().unwrap().query(
        &format!(
            "UPDATE process_status SET status = 'done' WHERE crate_id = '{}';",
            crate_id
        ),
        &[],
    )?;

    remove_dir_all(&format!("on_process/{}", name))?;
    conn.lock().unwrap().query(&query, &[])?;
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
