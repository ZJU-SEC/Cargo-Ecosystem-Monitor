use anyhow::{Result, anyhow};
use lazy_static::lazy_static;
use lifetime_hashmap::RUSTC_VER_NUM;
use regex::Regex;
use ansi_term::Color;

use std::{
    collections::{ HashSet, HashMap},
    process::Command,
};

use crate::{
    feature::{Feature, FEATURE_STORAGE},
    Cli, rustc_version::get_nightly_versions,
};

lazy_static! {
    static ref FEATS: FEATURE_STORAGE = FEATURE_STORAGE::new();
    static ref RUSTUP: HashMap<usize, String> = get_nightly_versions();
}

pub fn run(cli: Cli) -> Result<bool> {
    prebuild()?;
    while let Some((feasible_vers, faild_feats)) = local_cargo_build(&cli.features)? {
        // println!("Failed feature found: {:?}", faild_feats);
        if faild_feats.is_empty() {
            // No failed feature to fix
            // might be other errors
            return Ok(false);
        }
        for f in faild_feats {
            println!("{} {}", Color::Yellow.paint("[RUF Failure]"), f.name);
        }

        if feasible_vers.is_empty() {
            return Err(anyhow!("No possible solution found"));
            // No feasible version found
        } else {
            let mut max = 0;
            feasible_vers.iter()
                .map(|f| {
                    if f > &max {
                        max = *f;
                    }
                })
                .count();
            // switch toolchain version
            println!("[Try to fix] Switching channel: 1.{}.0", max);
            switch_toolchain(max)?;
        }
    }

    Ok(true)
}

fn prebuild() -> Result<()> {
    let res = Command::new("rustup").args(&["override", "set", "nightly"]).output()?;
    if !res.status.success() {
        return Err(anyhow!("Prebuild failed, nightly rustc needed."));
    }
    Ok(())
}

fn local_cargo_build(pfs: &Vec<String>) -> Result<Option<(HashSet<usize>, Vec<Feature>)>> {
    let res = if pfs.is_empty() {
        Command::new("cargo").args(&["build"]).output()
    } else {
        Command::new("cargo")
            .args(&["build", "--features"])
            .args(pfs)
            .output()
    }?;

    let mut vers = HashSet::from_iter(0..=RUSTC_VER_NUM);
    let mut feats = Vec::new();

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
            feats.push(feat);
        }

        return Ok(Some((vers, feats)));
    } else {
        return Ok(None);
    }
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

fn switch_toolchain(ver: usize) -> Result<()> {
    let err = anyhow!("Fetch needed version nightly-1.{}.0 failed", ver);
    if let Some(date) = RUSTUP.get(&ver) {
        let res = Command::new("rustup")
            .args(&["install", &format!("nightly-{}", date)])
            .output()?;
        if !res.status.success() {
            return Err(err);
        }

        let res = Command::new("rustup")
            .args(&["override", "set", &format!("nightly-{}", date)])
            .output()?;

        if !res.status.success() {
            return Err(err);
        }
    } else {
        return Err(err);
    }

    Ok(())
}