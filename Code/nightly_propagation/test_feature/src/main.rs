#![feature(exclusive_range_pattern)]

use postgres::{Client, NoTls};
use std::sync::{Arc, Mutex};
use util::{extract_info, prebuild};

extern crate downloader;
extern crate flate2;
extern crate lazy_static;
extern crate regex;
extern crate tar;
extern crate tidy;
extern crate walkdir;

mod util;

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


link: https://github.com/rust-lang/rust/archive/refs/tags/1.0.0.tar.gz
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
    1.41.0 ~  1.62.0    #[stable(feature = "xxx"[, since = "1.0.0"])]
                        #[unstable(feature = "xxx"[, reason = "xxx"][, issue = "xxx"])]
                        #[rustc_const_unstable(feature = "xxx"[, issue = "xxx"][, reason = "xxx"])]
                        #[rustc_const_stable(feature = "xxx"[, since = "1.0.0"])]
*/

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

    for v in 0..=63 {
        conn.lock()
            .unwrap()
            .query(
                &format!("ALTER TABLE feature_timeline ADD COLUMN v1_{}_0 VARCHAR", v),
                &[],
            )
            .unwrap_or_default();

        let res = extract_info(v);
        let mut features = vec![];

        features.extend(res.lang);
        features.extend(res.lib);

        features
            .into_iter()
            .map(|feat| {
                let exist = !conn
                    .lock()
                    .unwrap()
                    .query(
                        &format!("SELECT * FROM feature_timeline WHERE name = '{}'", feat.name),
                        &[],
                    )
                    .expect("Check exist fails")
                    .is_empty();
                if exist {
                    conn.lock()
                        .unwrap()
                        .query(
                            &format!(
                                "UPDATE feature_timeline SET v1_{}_0 = '{}' WHERE name = '{}'",
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
                                "INSERT INTO feature_timeline (name, v1_{}_0) VALUES ('{}', '{}')",
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

#[test]
fn test() {
    use std::path::Path;
    use tidy::features::check;
    let version = 48;

    run_my(version);
    run_tidy(version);

    fn run_my(version: i32) {
        let my_feats = extract_info(version);

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
                "CREATE TABLE IF NOT EXISTS public.feature_status_my (
        name VARCHAR,
        status VARCHAR,
        type VARCHAR
    )",
                &[],
            )
            .unwrap();

        for feat in my_feats.lang {
            conn.lock()
                .unwrap()
                .query(
                    &format!(
                        "INSERT INTO feature_status_my VALUES ('{}', '{}', '{}')",
                        feat.name,
                        feat.status.to_string(),
                        "lang"
                    ),
                    &[],
                )
                .unwrap();
        }

        for feat in my_feats.lib {
            conn.lock()
                .unwrap()
                .query(
                    &format!(
                        "INSERT INTO feature_status_my VALUES ('{}', '{}', '{}')",
                        feat.name,
                        feat.status.to_string(),
                        "lib"
                    ),
                    &[],
                )
                .unwrap();
        }
    }

    fn run_tidy(version: i32) {
        let src_path = format!("on_process/rust-1.{}.0/src", version);
        let compiler_path = format!("on_process/rust-1.{}.0/compiler", version);
        let lib_path = format!("on_process/rust-1.{}.0/library", version);
        let mut flag = false;

        let tidy_feats = check(
            Path::new(&src_path),
            Path::new(&compiler_path),
            Path::new(&lib_path),
            &mut flag,
            false,
        );

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
                "CREATE TABLE IF NOT EXISTS public.feature_status_tidy (
            name VARCHAR,
            status VARCHAR,
            type VARCHAR
            )",
                &[],
            )
            .unwrap();
        
        for feat in tidy_feats.lang {
            conn.lock()
                .unwrap()
                .query(
                    &format!(
                        "INSERT INTO feature_status_tidy VALUES ('{}', '{}', '{}')",
                        feat.0,
                        feat.1.level.to_string(),
                        "lang"
                    ),
                    &[],
                )
                .unwrap();
        }

        for feat in tidy_feats.lib {
            conn.lock()
                .unwrap()
                .query(
                    &format!(
                        "INSERT INTO feature_status_tidy VALUES ('{}', '{}', '{}')",
                        feat.0,
                        feat.1.level.to_string(),
                        "lib"
                    ),
                    &[],
                )
                .unwrap();
        }
    }
}
