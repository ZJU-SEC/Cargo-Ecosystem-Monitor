use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::Command;

use cargo::core::compiler::{CompileKind, RustcTargetData};
use cargo::core::resolver::{CliFeatures, ForceAllTargets, HasDevUnits};
use cargo::core::{PackageIdSpec, Workspace};
use cargo::ops::{self, WorkspaceResolve};
use cargo::util::Config;

use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use regex::Regex;
use toml::Value;
use walkdir::WalkDir;

use crate::RUSTC;

#[derive(Hash, Eq, PartialEq, Debug)]
pub struct Feature {
    name: String,
    cfg: String,
}

pub fn resolve(path: &str, name: &str) -> Result<HashSet<Feature>> {
    // let mut features = Vec::new();
    // let config = Config::new(Shell::new(), env::current_dir()?, path.into());
    let config = Config::default()?;

    let ws = Workspace::new(&Path::new(&format!("{path}/Cargo.toml")), &config)?;
    let requested_targets = Vec::new();
    let requested_kinds = CompileKind::from_requested_targets(ws.config(), &requested_targets)?;
    let target_data = RustcTargetData::new(&ws, &requested_kinds)?;

    let specs = PackageIdSpec::query_str(name, ws.members().map(|pkg| pkg.package_id()))?;
    let specs = PackageIdSpec::from_package_id(specs);

    let resolve = ops::resolve_ws_with_opts(
        &ws,
        &target_data,
        &requested_kinds,
        &CliFeatures::new_all(false),
        &[specs],
        HasDevUnits::Yes,
        ForceAllTargets::No,
    )?;
    
    fetch_features(&resolve, name)
}

fn fetch_features(resolve: &WorkspaceResolve, root: &str) -> Result<HashSet<Feature>> {
    // let root = resolve.query(root)?;
    // let deps: Vec<PackageId> = resolve.deps(root).map(|(pkg, _)| pkg).collect();
    // println!("{:?}", root);
    // if deps.is_empty() {
    //     return fetch_package_features(root.source_id());
    // } else {
    //     let mut set = HashSet::new();
    //     for dep in deps {
    //         let features = fetch_features(resolve, &dep.name())?;
    //         set.extend(features.into_iter());
    //     }
    //     return Ok(set);
    // }
    let mut set = HashSet::new();
    for pkg in resolve.pkg_set.packages() {
        set.extend(fetch_package_features(pkg.manifest_path())?);
    }

    Ok(set)
}

fn fetch_package_features(path: &Path) -> Result<HashSet<Feature>> {
    let mut set = HashSet::new();
    // let mut ori_features = vec![];
    // let mut pro_features = vec![];
    let mut to_dos = vec![];
    let mut edition = String::from("2015");

    for entry in WalkDir::new(&path) {
        let entry = entry?;

        if entry
            .path()
            .file_name()
            .unwrap()
            .eq_ignore_ascii_case("lib.rs")
        {
            to_dos.push(entry.path().to_owned());
        } else if entry
            .path()
            .file_name()
            .unwrap()
            .eq_ignore_ascii_case("Cargo.toml")
        {
            let mut file = File::open(entry.path())?;
            let mut buf = String::new();
            file.read_to_string(&mut buf).unwrap();

            let toml = buf.parse::<Value>()?;
            edition = toml
                .get("package")
                .map(|v| {
                    v.get("edition")
                        .map(|v| v.as_str().unwrap_or("2015"))
                        .unwrap_or("2015")
                })
                .unwrap_or("2015")
                .to_owned();
        }
    }

    for librs in &mut to_dos {
        let exec = Command::new(RUSTC)
            .arg("--edition")
            .arg(&edition)
            .arg("--ruf-analysis")
            .arg(&librs)
            .output()?;

        if exec.status.success() {
            let out = String::from_utf8(exec.stdout).unwrap();

            lazy_static! {
                static ref RE1: Regex = Regex::new(r#"formatori \(\[(.*?)\], (.*?)\)"#).unwrap();
                static ref RE2: Regex = Regex::new(r#"processed \(\[(.*?)\], (.*?)\)"#).unwrap();
            }

            RE1.captures_iter(&out)
                .map(|cap| {
                    if let (Some(cond), Some(feat)) = (cap.get(1), cap.get(2)) {
                        set.insert(Feature {
                            name: feat.as_str().to_string(),
                            cfg: cond.as_str().to_string(),
                        });
                        // ori_features.push((cond.as_str().to_string(), feat.as_str().to_string()));
                    }
                })
                .count();

            // RE2.captures_iter(&out)
            //     .map(|cap| {
            //         if let (Some(cond), Some(feat)) = (cap.get(1), cap.get(2)) {
            //             pro_features.push((cond.as_str().to_string(), feat.as_str().to_string()));
            //         }
            //     })
            //     .count();
        } else {
            return Err(anyhow!(
                "Resolve {:?} feature fails",
                &path.file_name().unwrap()
            ));
        }
    }

    Ok(set)
}
