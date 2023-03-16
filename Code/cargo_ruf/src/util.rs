use anyhow::Result;
use lazy_static::lazy_static;
use lifetime_hashmap::RUSTC_VER_NUM;
use regex::Regex;

use std::{collections::HashSet, process::Command};

use crate::{feature::Feature, Cli};

pub fn run(cli: Cli) {
    let res = local_cargo_build(cli.features).unwrap_or_default();
    println!("{:?}", res);
}

fn local_cargo_build(pfs: Vec<String>) -> Result<HashSet<usize>> {
    let res = if pfs.is_empty() {
        Command::new("cargo").args(&["build"]).output()
    } else {
        Command::new("cargo")
            .args(&["build", "--features"])
            .args(pfs)
            .output()
    }?;

    let mut vers = HashSet::from_iter(0..=RUSTC_VER_NUM);

    // build fails
    if !res.status.success() {
        // error\[E(\d+)\]:(.+)[^.]\s+-->.+:\d+:(\d+)[^.]\s+\|.*[^.].+\|(.+)
        lazy_static! {
            static ref RE: Regex = Regex::new(r#"error\[E(?P<eid>\d+)\]:.+[^.] -->.+:\d+:(?P<col>\d+)[^.]\s+\|.*[^.].+\|(?P<line>.+)"#)
                .expect("Fatal: Init regex failed");
        };
        let err = String::from_utf8_lossy(&res.stderr);
        // form errors
        let errs = RE
            .captures_iter(&err)
            .into_iter()
            .map(|cap| {
                (
                    cap["eid"].parse::<usize>().unwrap(),
                    cap["col"].parse::<usize>().unwrap(),
                    cap["line"].to_string(),
                )
            })
            .collect();

        // find feasible versions
        for mut feat in resolve_build_errors(errs) {
            feat.sync_metas();
            // println!("{:?}", feat);
            vers = vers.intersection(&feat.usable()).cloned().collect();
        }
    }

    Ok(vers)
}

// 635 - unknown feature
// 557 - removed feature
fn resolve_build_errors(errs: Vec<(usize, usize, String)>) -> Vec<Feature> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r#"(?P<name>\w+)"#).expect("Fatal: Init regex failed");
    };

    // println!("{:?}", errs);
    let mut err_features = HashSet::new();
    for (eid, col, cont) in errs {
        if eid == 635 || eid == 557 {
            let name = RE
                .captures(&cont[col..])
                .expect("Fatal: Fetch feature name failed")["name"]
                .to_string();
            err_features.insert(name);
        }
    }
    // println!("{:?}", err_features);

    err_features.into_iter().map(|f| Feature::new(f)).collect()
}
