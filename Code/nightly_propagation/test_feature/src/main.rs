#![feature(exclusive_range_pattern)]

extern crate downloader;
extern crate flate2;
extern crate lazy_static;
extern crate regex;
extern crate simplelog;
extern crate tar;
extern crate walkdir;

mod util;

use log::warn;
use postgres::{Client, NoTls};
use simplelog::*;
use std::{
    sync::{Arc, Mutex},
    fs::OpenOptions,
};

#[allow(unused)]
use util::download_info;
use util::{extract_info, prebuild};

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
    \("([a-zA-Z0-9_]+?)", .+, (Active|Accepted|Removed|Incomplete)\)
    \((active|accepted|removed|incomplete), ([a-zA-Z0-9_]+?), .+\)


link: https://github.com/rust-lang/rust/tags
*/

/*
check:
    1.0.0  ~  1.2.0     #[stable(feature = "xxx"[, since = "1.0.0"])]
                        #[unstable(feature = "xxx"[, reason = "xxx"])]
    1.3.0  ~  1.24.0    #[stable(feature = "xxx"[, since = "1.0.0"])]
                        #[unstable(feature = "xxx"[, reason = "xxx"][, issue = "xxx"])]
    1.25.0 ~  1.40.0    #[stable(feature = "xxx"[, since = "1.0.0"])]
                        #[unstable(feature = "xxx"[, reason = "xxx"][, issue = "xxx"])]
                        #[rustc_const_unstable(feature = "xxx")]
    1.41.0 ~  1.67.0    #[stable(feature = "xxx"[, since = "1.0.0"])]
                        #[unstable(feature = "xxx"[, reason = "xxx"][, issue = "xxx"])]
                        #[rustc_const_unstable(feature = "xxx"[, issue = "xxx"][, reason = "xxx"])]
                        #[rustc_const_stable(feature = "xxx"[, since = "1.0.0"])]
*/

fn main() {
    // enable if rustc versions not downloaded
    println!("Download Compiler Source Code ...");
    download_info();
    return;
    let conn = Arc::new(Mutex::new(
        Client::connect(
            "host=localhost dbname=crates user=postgres password=postgres",
            NoTls,
        )
        .unwrap(),
    ));

    // Prepare log file
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Warn,
            simplelog::Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Warn,
            simplelog::Config::default(),
            OpenOptions::new()
                .write(true)
                .create(true)
                .open("./feature_lifetime.log")
                .unwrap(),
        ),
    ])
    .unwrap();

    println!("Prebuild Database ...");
    prebuild(Arc::clone(&conn));

    println!("Extracting RUF Lifetime ...");
    for v in 0..=67 {
        conn.lock()
            .unwrap()
            .query(
                &format!("ALTER TABLE feature_timeline ADD COLUMN v1_{v}_0 VARCHAR"),
                &[],
            )
            .unwrap_or_default();

        let res = extract_info(v);
        let mut features = vec![];

        features.extend(res.lang);
        features.extend(res.lib);

        warn!("[warn] nightly-1.{}.0 multi status features: {:?}", v, res.mul_errors);
        warn!("[warn] nightly-1.{}.0 nr error features: {:?}", v, res.nr_errors);

        // continue;
        features
            .into_iter()
            .map(|feat| {
                let exist = !conn
                    .lock()
                    .unwrap()
                    .query(
                        &format!(
                            "SELECT * FROM feature_timeline WHERE name = '{}'",
                            feat.name
                        ),
                        &[],
                    )
                    .expect("Check exist fails")
                    .is_empty();

                // let status = if feat.rustc_const {
                //     format!("{} (rustc_const)", feat.status.to_string())
                // } else {
                //     feat.status.to_string()
                // };

                let status = feat.status.to_string();

                if exist {
                    conn.lock()
                        .unwrap()
                        .query(
                            &format!(
                                "UPDATE feature_timeline SET v1_{}_0 = '{}' WHERE name = '{}'",
                                v, status, feat.name
                            ),
                            &[],
                        )
                        .expect("update fails");
                } else {
                    conn.lock()
                        .unwrap()
                        .query(
                            &format!(
                                "INSERT INTO feature_timeline (name, v1_{}_0) VALUES ('{}', '{}')",
                                v, feat.name, status
                            ),
                            &[],
                        )
                        .expect("insert fails");
                }
            })
            .count();
    }
}

/*
#[test]
fn test() {
    let mut normal_set_all = HashSet::new();
    let mut rc_set_all = HashSet::new();
    let mut multi_status_features_all = HashSet::new();

    for v in 0..=67 {
        let mut multi_status_features = HashSet::new();
        let res = extract_info(v, &mut multi_status_features);

        let mut features = vec![];

        features.extend(res.lang);
        features.extend(res.lib);

        let mut normal_set = HashSet::new();
        let mut rc_set = HashSet::new();

        for feat in features {
            if feat.rustc_const {
                rc_set.insert(feat.name);
            } else {
                normal_set.insert(feat.name);
            }
        }

        let x = normal_set.intersection(&rc_set);
        println!("NR Error: {:?}", x);
        println!("MultiStatus Error: {:?}", multi_status_features);

        normal_set_all.extend(normal_set);
        rc_set_all.extend(rc_set);
        multi_status_features_all.extend(multi_status_features);
    }

    let x = normal_set_all.intersection(&rc_set_all);
    println!("NR Error All: {:?}", x);
    println!("MultiStatus Error All: {:?}", multi_status_features_all);

}

*/
