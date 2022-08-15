

pub mod tools;

const THREADNUM:usize = 2; // Thread number
const CRATES_NUM:i64 = 1000; // Evaluated crate sample number

use std::collections::{HashSet, HashMap};
use std::fs::{create_dir, remove_dir_all, File, OpenOptions};
use std::panic::{self, catch_unwind};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::process::Command;
use std::io::prelude::*;

use simplelog::*;
use anyhow::{ Result};
use crossbeam::channel::{self};
use downloader::{Downloader};
use log::{error, info, warn};
use pbr::MultiBar;
use postgres::{Client, NoTls};
use regex::Regex;

use tools::db::*;
use tools::helper::*;
#[test]
fn from_dependency_file_test() -> Result<()>{
    let str = "output/dependency/abel-core-cargotree.csv";
    let dependencies = from_dependency_file(str.to_string())?;
    println!("dependencies:{:?}", dependencies);
    Ok(())
}

#[test]
fn cargo_lock_resolution(){
    create_dir(Path::new(CRATESDIR)).unwrap_or_default();
    let mut downloader = Downloader::builder()
                    .download_folder(Path::new(CRATESDIR))
                    .parallel_requests(1)
                    .build()
                    .expect("Fatal Error, build downloader fails!");
    let name = "coreutils";
    let version = "0.0.14";
    // Download crate source code
    if let Err(e) = fetch_crate(&mut downloader, &name, &version) {
       println!("Fetch fails: crate {}/{}, {}", name, version, e);
    } else {
        println!("Fetch Success: crate {}/{}", name, version);
        // Decompress
        let output = Command::new("tar").arg("-zxf")
                           .arg(format!("{}/{}/{}.tgz", CRATESDIR, name, version))
                           .arg("-C")
                           .arg(format!("{}/{}", CRATESDIR, name))
                           .output().expect("Ungzip exec error!");
        let output_str = String::from_utf8_lossy(&output.stdout);
        println!("Decompress {}/{}/{}.tgz success: {}", CRATESDIR, name, version, output_str);
        
        // Create newest lock file (Remove lock file to keep it up-to-date)
        let output = Command::new("rm").arg(format!("{}/{}/{}-{}/Cargo.lock", CRATESDIR, name, name, version))
                                      .output().expect("rm Cargo.lock exec error!");
        let output_str = String::from_utf8_lossy(&output.stdout);
        println!("Remove Cargo.lock {}/{}/{}-{}/Cargo.lock success: {}", CRATESDIR, name, name, version, output_str);
        Command::new("cargo").arg("tree")
                             .arg("--manifest-path")
                             .arg(format!("{}/{}/{}-{}/Cargo.toml", CRATESDIR, name, name, version)) // toml path
                             .output().expect("tree exec error!");
        
                             // Extract each dependency, deduplicate and store in csv format
        let lockfile_path = format!("{}/{}/{}-{}/Cargo.lock", CRATESDIR, name, name, version);
        let mut lockfile = OpenOptions::new()
                        .read(true)
                        .open(lockfile_path)
                        .expect("Can't open Cargo.lock file");
        let mut lockdep = String::new();
        lockfile.read_to_string(&mut lockdep).expect("Read Cargo.lock file fail");
        println!("Content: {}", lockdep);
        
        // Create File
        let path_string = format!("{}/{}/dependencies.csv", CRATESDIR, name);
        let path = Path::new(path_string.as_str());
        let display = path.display();   
        let mut file = match File::create(&path) {
            Err(why) => panic!("couldn't create {}: {}", display, why),
            Ok(file) => file,
        };
        
        // Use regular expression to resolve Cargo.lock file.
        let mut dependencies = HashSet::new();
        let re = Regex::new(concat!(
            r#"name = "[\S]+""#, // Crate Name
            "\n",
            r#"version = "[0-9]+.[0-9]+.[0-9]+[\S]*""#, // Crate version
        )).unwrap();
        for cap in re.captures_iter(&lockdep) {
            let dep = &cap[0];
            let name_ver:Vec<&str> = dep.split('\n').collect();
            // TODO: Format dep_name and dep_ver
            let v_name: Vec<&str> = name_ver[0].rsplit('"').collect(); 
            let v_ver: Vec<&str> = name_ver[1].rsplit('"').collect(); 
            //After splition, the second str is the name/version
            let dep_name = format!("\"{}\",", v_name[1]);
            let dep_ver = format!("\"{}\"", v_ver[1]);
            dependencies.insert(dep_name+ &dep_ver);
        }
        
        // dependencies.remove(&format!("\"{}\",\"{}\"", name, version)); // Remove current version
        let mut sorted = dependencies.into_iter().collect::<Vec<_>>();
        sorted.sort();
        println!("Dependency:{:?}",sorted);
        for mut dependency in sorted {
            dependency.push('\n');
            if let Err(why)=file.write_all(dependency.replace(" ", ",").as_bytes()) {
                panic!("couldn't write to {}: {}", display, why);
            }
        }
    }
}
