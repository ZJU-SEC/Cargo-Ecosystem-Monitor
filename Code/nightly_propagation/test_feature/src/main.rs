#![feature(exclusive_range_pattern)]

extern crate downloader;
extern crate flate2;
extern crate lazy_static;
extern crate regex;
extern crate tar;

use downloader::{Download, Downloader};
use flate2::read::GzDecoder;
use lazy_static::lazy_static;
use postgres::{Client, NoTls};
use regex::Regex;
use std::collections::HashMap;
use std::fs::{create_dir, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};
use tar::Archive;

/*
check:
    src/libsyntax/feature_gate.rs

    1.0.0  ~ 1.3.0  ("globs", "1.0.0", Accepted)
    1.4.0  ~ 1.9.0  ("simd", "1.0.0", Some(27731), Active)
    1.10.0 ~ 1.25.0 (active, simd, "1.0.0", Some(27731))
    1.26.0 ~ 1.38.0 (active, asm, "1.0.0", Some(29722), None)

    src/libsyntax/feature_gate/xxx.rs
    1.39.0 ~ 1.40.0 (active, rustc_private, "1.0.0", Some(27812), None)

    src/librustc_feature/xxx.rs
    1.41.0 ~ 1.47.0 (active, rustc_private, "1.0.0", Some(27812), None)

    compiler/rustc_feature/src/xxx.rs
    1.48.0 ~ 1.62.0 (active, rustc_private, "1.0.0", Some(27812), None)

regex:
    \("([a-zA-Z0-9_]+?)", .+, (Active|Accepted|Removed)\)
    \((active|accepted|removed), ([a-zA-Z0-9_]+?), .+\)


link: https://github.com/rust-lang/rust/archive/refs/tags/1.0.0.tar.gz
*/

#[derive(Debug)]
struct Feature {
    name: String,
    status: Status,
}

#[derive(Debug, Clone, Copy)]
enum Status {
    Active,
    Accepted,
    Removed,
    Unknown,
}

fn main() {
    // enable if rustc versions not downloaded
    // download_info();

    let conn = Arc::new(Mutex::new(
        Client::connect(
            "host=localhost dbname=crates_08_22 user=postgres password=postgres",
            NoTls,
        )
        .unwrap(),
    ));

    prebuild(Arc::clone(&conn));

    for v in 0..=62 {
        conn.lock()
            .unwrap()
            .query(
                &format!("ALTER TABLE feature_status ADD COLUMN v1_{}_0 VARCHAR", v),
                &[],
            )
            .unwrap_or_default();

        extract_info(v)
            .into_iter()
            .map(|feat| {
                let exist = !conn
                    .lock()
                    .unwrap()
                    .query(
                        &format!("SELECT * FROM feature_status WHERE name = '{}'", feat.name),
                        &[],
                    )
                    .expect("Check exist fails")
                    .is_empty();
                if exist {
                    conn.lock()
                        .unwrap()
                        .query(
                            &format!(
                                "UPDATE feature_status SET v1_{}_0 = '{}' WHERE name = '{}'",
                                v,
                                feat.status.to_string(),
                                feat.name
                            ),
                            &[],
                        )
                        .expect("update fails");
                } else {
                    conn.lock()
                        .unwrap()
                        .query(
                            &format!(
                                "INSERT INTO feature_status (name, v1_{}_0) VALUES ('{}', '{}')",
                                v,
                                feat.name,
                                feat.status.to_string()
                            ),
                            &[],
                        )
                        .expect("insert fails");
                }
            })
            .count();
    }
}

fn prebuild(conn: Arc<Mutex<Client>>) {
    conn.lock()
        .unwrap()
        .query(
            r#"CREATE TABLE IF NOT EXISTS public.feature_status (name VARCHAR)"#,
            &[],
        )
        .unwrap();

    conn.lock()
        .unwrap()
        .query(
            r#"INSERT INTO feature_status (
            SELECT DISTINCT feature FROM version_feature)"#,
            &[],
        )
        .unwrap();

    conn.lock()
        .unwrap()
        .query(
            r#"DELETE FROM feature_status WHERE name = 'no_feature_used'"#,
            &[],
        )
        .unwrap()
}

#[allow(unused)]
fn download_info() {
    create_dir("on_progress").unwrap_or_default();

    let mut downloader = Downloader::builder()
        .download_folder(Path::new("on_process"))
        .parallel_requests(4)
        .build()
        .expect("Fatal Error, build downloader fails!");

    let mut dls = Vec::new();

    for ver in 0..=62 {
        dls.push(Download::new(&format!(
            "https://github.com/rust-lang/rust/archive/refs/tags/1.{}.0.tar.gz",
            ver
        )));
    }

    downloader.download(&dls).expect("downloader broken");
}

fn extract_info(ver: i32) -> Vec<Feature> {
    let ver_str = format!("1.{}.0", ver);
    let data = File::open(&format!("on_process/{}.tar.gz", ver_str)).expect("Open file failed");
    let mut archive = Archive::new(GzDecoder::new(data));
    let mut features = Vec::new();

    lazy_static! {
        static ref RE1: Regex =
            Regex::new(r#"\("([a-zA-Z0-9_]+?)", .+, (Active|Accepted|Removed)\)"#).unwrap();
    }

    lazy_static! {
        static ref RE2: Regex =
            Regex::new(r#"\((active|accepted|removed), ([a-zA-Z0-9_]+?), .+\)"#).unwrap();
    }

    if File::open(&format!("on_process/{}", ver_str)).is_err() {
        println!("Unpacked {}", ver_str);
        archive.unpack("on_process/").expect("Unpack file failed");
    }

    match ver {
        0..=38 => {
            let mut file = OpenOptions::new()
                .read(true)
                .open(&format!(
                    "on_process/rust-{}/src/libsyntax/feature_gate.rs",
                    ver_str
                ))
                .expect("Open file failed");

            let mut content = String::new();
            file.read_to_string(&mut content).expect("Read file failed");

            if ver <= 9 {
                for cap in RE1.captures_iter(&content) {
                    features.push(Feature {
                        name: cap[1].to_string(),
                        status: cap[2].to_ascii_lowercase().into(),
                    });
                }
            } else {
                for cap in RE2.captures_iter(&content) {
                    features.push(Feature {
                        name: cap[2].to_string(),
                        status: cap[1].to_ascii_lowercase().into(),
                    });
                }
            }
        }
        39..=62 => {
            let pre_path = match ver {
                39..=40 => "src/libsyntax/feature_gate",
                41..=47 => "src/librustc_feature",
                48..=62 => "compiler/rustc_feature/src",
                _ => unreachable!(),
            };

            let mut content = String::new();

            for name in ["accepted.rs", "active.rs", "removed.rs"] {
                let mut file = OpenOptions::new()
                    .read(true)
                    .open(&format!(
                        "on_process/rust-{}/{}/{}",
                        ver_str, pre_path, name
                    ))
                    .expect("Open file failed");

                let mut buf = String::new();
                file.read_to_string(&mut buf).expect("Read file failed");
                content.push_str(&buf);
            }

            for cap in RE2.captures_iter(&content) {
                features.push(Feature {
                    name: cap[2].to_string(),
                    status: cap[1].to_ascii_lowercase().into(),
                });
            }
        }
        _ => unreachable!(),
    }

    features
}

impl From<String> for Status {
    fn from(s: String) -> Self {
        match s.as_str() {
            "active" => Status::Active,
            "accepted" => Status::Accepted,
            "removed" => Status::Removed,
            _ => Status::Unknown,
        }
    }
}

impl ToString for Status {
    fn to_string(&self) -> String {
        match self {
            Status::Active => "active",
            Status::Accepted => "accepted",
            Status::Removed => "removed",
            Status::Unknown => "unknown",
        }
        .to_string()
    }
}

#[test]
fn test() {
    let res = extract_info(62);
    println!("{:?}", res);
}

/*
fn legacy_run() {
    // set up
    let set_up_res = Command::new("sh")
        .arg("src/setup.sh")
        .output()
        .expect("failed to execute process");
    if !set_up_res.status.success() {
        panic!("failed to set up");
    }

    let conn = Arc::new(Mutex::new(
        Client::connect(
            "host=localhost dbname=crates user=postgres password=postgres",
            NoTls,
        )
        .unwrap(),
    ));

    prebuild(Arc::clone(&conn));

    let features: Vec<String> = conn
        .lock()
        .unwrap()
        .query("SELECT DISTINCT feature FROM version_feature;", &[])
        .unwrap()
        .into_iter()
        .map(|feat| feat.get(0))
        .collect();

    for feat in features {
        let (status, info) = do_test(&feat);
        conn.lock()
            .unwrap()
            .query(
                &format!(
                    "INSERT INTO feature_status VALUES('{}', '{}', '{}')",
                    feat, status, info
                ),
                &[],
            )
            .unwrap();

        println!("{} {}", feat, status);
    }
}

fn do_test(feature: &str) -> (String, String) {
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open("do_test.rs")
        .expect("Open file fails");

    let buf = format!("#![feature({})]\nfn main() {{}}", feature);
    file.write_all(&buf.as_bytes()).expect("Write file fails");

    let res = Command::new("rustc")
        .arg("do_test.rs")
        .output()
        .expect("failed to execute process");

    let stderr = String::from_utf8(res.stderr).expect("resolve rustc result fails");

    if stderr.len() == 0 {
        if res.status.success() {
            return ("ok".to_string(), "".to_string());
        } else {
            return ("others".to_string(), "unexpected fails".to_string());
        }
    } else {
        if stderr.contains("has been stable") {
            return ("stablized".to_string(), stderr);
        } else if stderr.contains("unknown feature") {
            return ("unknown".to_string(), stderr);
        } else if stderr.contains("has been removed") {
            return ("removed".to_string(), stderr);
        } else if stderr.contains("incomplete") {
            return ("incomplete".to_string(), stderr);
        } else {
            return ("others".to_string(), stderr);
        }
    }
}

*/
