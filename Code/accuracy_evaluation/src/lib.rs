

pub mod tools;

pub const THREADNUM:usize = 8; // Thread number
pub const CRATES_NUM:i64 = 1000; // Evaluated crate sample number

use std::collections::{HashSet, HashMap};
use std::fs::{create_dir, remove_dir_all, File, OpenOptions};
use std::panic::{self, catch_unwind};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::process::Command;
use std::io::prelude::*;
use std::io::BufReader;

use simplelog::*;
use anyhow::{anyhow, Result};
use crossbeam::channel::{self};
use downloader::{Downloader, Download};
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
    let name = "slab";
    let version = "0.4.7";
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
        // println!("Content: {}", lockdep);
        
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


const DEBUGDIR:&str = "debug";

#[test]
/// This is used to find difference between pipeline and cargo tree resolution.
/// It outputs contents as follows:
/// 1. Crate id and version id
/// 2. Crates.io Web Url
/// 3. Source Code (Including newest Cargo.lock)
/// 4. Sorted Cargo tree Resolution Results
/// 5. Sorted Pipeline Resolution Results
/// 6. Dependency Analysis results
/// 7. Pipeline Resolve Error, if exists.
fn display_full_information_of_crate() -> Result<()>{
    let name = "finchers-ext";
    let version = "0.11.0";
    let conn = Arc::new(Mutex::new(
        Client::connect(
            "host=localhost dbname=crates user=postgres password=postgres",
            NoTls,
        )
        .unwrap(),
    ));


    // 1. Crate id and version id
    let crate_id:i32 = conn.lock().unwrap()    
                                .query(
                                    &format!(r#"SELECT id FROM crates WHERE name = '{}'"#, name),
                                    &[],
                                ).unwrap().first().unwrap().get(0);
    println!("crate_id = {}", crate_id);
    let version_id:i32 = conn.lock().unwrap()    
                                .query(
                                    &format!(r#"SELECT id FROM versions WHERE crate_id = {} AND num = '{}'"#
                                    , crate_id, version),
                                    &[],
                                ).unwrap().first().unwrap().get(0);
    println!("version_id = {}", version_id);

    // 2. Crates.io Web Url
    let url = format!("https://crates.io/crates/{}/{}", name, version);
    println!("url = {}", url);


    // 3. Source Code (Including newest Cargo.lock)
    remove_dir_all(DEBUGDIR).unwrap_or_default();
    create_dir(Path::new(DEBUGDIR)).unwrap_or_default();
    let mut downloader = Downloader::builder()
                    .download_folder(Path::new(DEBUGDIR))
                    .parallel_requests(1)
                    .build()
                    .expect("Fatal Error, build downloader fails!");
    let mut dls = vec![];
    create_dir(Path::new(&format!("{}/{}", DEBUGDIR, name))).unwrap_or_default();
    dls.push(
        Download::new(&format!(
            "https://crates.io/api/v1/crates/{}/{}/download",
            name, version
        ))
        .file_name(Path::new(&format!("{}/{}.tgz", name, version))),
    );
    let res = downloader.download(&dls)?;
    if res.iter().any(|res| res.is_err()) {
        return Err(anyhow!("Download error."));
    }
    // Decompress
    let output = Command::new("tar").arg("-zxf")
                                    .arg(format!("{}/{}/{}.tgz", DEBUGDIR, name, version))
                                    .arg("-C")
                                    .arg(format!("{}/{}", DEBUGDIR, name))
                                    .output().expect("Ungzip exec error!");
    let output_str = String::from_utf8_lossy(&output.stdout);
    println!("Decompress {}/{}/{}.tgz success: {}", DEBUGDIR, name, version, output_str);
    // Create newest lock file (Remove lock file to keep it up-to-date)
    let output = Command::new("rm").arg(format!("{}/{}/{}-{}/Cargo.lock", DEBUGDIR, name, name, version))
                                    .output().expect("rm Cargo.lock exec error!");
    let output_str = String::from_utf8_lossy(&output.stdout);
    println!("Remove Cargo.lock {}/{}/{}-{}/Cargo.lock success: {}", DEBUGDIR, name, name, version, output_str);
    // Replace original file
    {
        let toml = File::open(Path::new(&format!("{}/{}/{}-{}/Cargo.toml", DEBUGDIR, name, name, version)))?;
        let mut buf_reader = BufReader::new(toml);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents)?;
        let re = Regex::new(r#"edition = "[0-9]+""#).unwrap();
        let mut edition:String = String::new();
        for cap in re.captures_iter(&contents) {
            edition = cap[0].to_string();
        }
        let new_content = if !edition.is_empty(){
            contents.replace(&edition, r#"edition = "2021""#)
        }
        else{
            contents.replace("[package]", "[package]\nedition = \"2021\"")
        };
        println!("File : {}", new_content);

        
        let mut toml = File::options().write(true).
            open(Path::new(&format!("{}/{}/{}-{}/Cargo.toml", DEBUGDIR, name, name, version)))?;
        toml.write_all(new_content.as_bytes())?;
        toml.sync_all()?;
    }
        
    // 4. Sorted Cargo tree Resolution Results
    // Run `cargo tree` in each target, get union of their dependency graph as final results.
    // Data structure of `dependencies`: HashMap<crate_name, HashSet<versions>>
    let mut cargotree_crates:HashMap<String, HashSet<String>> = HashMap::new();
    let re = Regex::new(r"[\w-]+ v[0-9]+.[0-9]+.[0-9]+[\S]*").unwrap();
    let output = Command::new("cargo").arg("tree")
                                        .arg("--manifest-path")
                                        .arg(format!("{}/{}/{}-{}/Cargo.toml", DEBUGDIR, name, name, version)) // toml path
                                        .arg("-e")
                                        .arg("no-dev")
                                        .arg("--all-features")
                                        .arg("--target")
                                        .arg("all")
                                        .output().expect("tree exec error!");
    let output_str = String::from_utf8_lossy(&output.stdout);
    // println!("dep: {}", output_str);
    for cap in re.captures_iter(&output_str) {
        let dep = &cap[0];
        let name_ver:Vec<&str> = dep.split(' ').collect();
        let dep_name = String::from(name_ver[0]);
        let mut dep_ver = String::from(name_ver[1]);
        dep_ver.remove(0); // Remove char 'v' at the beginning of dep_ver
        let crate_name = cargotree_crates.entry(dep_name).or_insert(HashSet::new());
        (*crate_name).insert(dep_ver);
    }
    let crate_name = cargotree_crates.entry(String::from(name)).or_insert(HashSet::new());
    (*crate_name).remove(&String::from(version)); // Remove current version
    let path_string = format!("{}/{}-{}-cargotree.csv", DEBUGDIR, name, version);
    write_dependency_file_sorted(path_string, &cargotree_crates);


    // 5. Sorted Pipeline Resolution Results
    let mut pipeline_crates:HashMap<String, HashSet<String>> = HashMap::new();
    let query = format!(
        "WITH target_dep AS(
            WITH target_version AS 
            (SELECT distinct version_to FROM dep_version
            WHERE version_from = {})
            SELECT crate_id, num FROM target_version INNER JOIN versions ON version_to = id)
            SELECT name, num FROM target_dep INNER JOIN crates ON crate_id = id ORDER BY num asc
        ", version_id
    );
    let row = conn.lock().unwrap().query(&query, &[]).unwrap();
    for ver in row {
        let dep_name = ver.get(0);
        let dep_ver = ver.get(1);
        let crate_name = pipeline_crates.entry(dep_name).or_insert(HashSet::new());
        (*crate_name).insert(dep_ver);
    }
    let path_string = format!("{}/{}-{}-pipeline.csv", DEBUGDIR, name, version);
    write_dependency_file_sorted(path_string, &pipeline_crates);
    if pipeline_crates.is_empty() {
        println!("Pipeline hasn't resolved it yet!");
    }


    // 6. Dependency Analysis results
    let cargotree_crates_num = get_dependency_num(&cargotree_crates);
    let pipeline_crates_num = get_dependency_num(&pipeline_crates);
    let mut overresolve_dep = 0;
    let mut right_dep = 0;
    let mut wrong_dep = 0;
    let mut missing_dep = 0;
    // Four types of results:
    // Over-resolve, Right/Wrong-resolve, Missing-resolve
    for(crate_name, versions) in &pipeline_crates {
        // Over-resolve: No such dep, but resolves.
        if !cargotree_crates.contains_key(crate_name){
            overresolve_dep += versions.len();
        }
        else{
            // Right/wrong-resolve: 
            // Compare results to see if resolution results is right
            for version in versions {
                if cargotree_crates[crate_name].contains(version){
                    right_dep += 1;
                }
                else{
                    wrong_dep += 1;
                }
            }
        }
    }
    // Missing-resolve: Crates exsits but is not resolved.
    for(crate_name, versions) in &cargotree_crates {
        if !pipeline_crates.contains_key(crate_name){
            missing_dep += versions.len();
        }
    }
    let line = format!("
        cargotree_crates_num = {}
        pipeline_crates_num = {}
        overresolve_dep = {}
        right_dep = {}
        wrong_dep = {}
        missing_dep = {}
        ", 
        cargotree_crates_num,
        pipeline_crates_num ,
        overresolve_dep,
        right_dep,
        wrong_dep,
        missing_dep
    );
    println!("Results: \n{}", line);


    // 7. Pipeline Resolve Error, if exists.
    let query = format!(
        "SELECT error FROM dep_errors WHERE ver = {}
        ", version_id
    );
    if let Some(row) = conn.lock().unwrap().query(&query, &[]).unwrap().first(){
        let error:String = row.get(0);
        println!("Pipeline Resolve Error: {}", error);
    }

    Ok(())
}