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

use accuracy_evaluation::tools::db::*;
use accuracy_evaluation::tools::helper::*;

const THREADNUM:usize = 2; // Thread number
const CRATES_NUM:i64 = 1000; // Evaluated crate sample number

fn main() {
    // Prepare log file
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Warn,
            simplelog::Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Info,
            simplelog::Config::default(),
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .append(true)
                .open("./benchmark_dataset.log")
                .unwrap(),
        ),
    ])
    .unwrap();

    // Prepare work directories
    
    remove_dir_all(CRATESDIR).unwrap_or_default();// Delete tmp crates file directory
    create_dir(Path::new(CRATESDIR)).unwrap_or_default(); // Crates file directory
    create_dir(Path::new(OUTPUTDIR)).unwrap_or_default();
    create_dir(Path::new(RESULTSDIR)).unwrap_or_default(); // Resolved results directory
    create_dir(Path::new(RAW_DEPENDENCYDIR)).unwrap_or_default(); // Resolved dependency directory
    create_dir(Path::new(CARGOTREE_DEPENDENCYDIR)).unwrap_or_default(); // Resolved dependency directory
    create_dir(Path::new(PIPELINE_DEPENDENCYDIR)).unwrap_or_default(); // Resolved dependency directory
    // Main Process
    run();
}



fn run(){
    // Get all possible rustup targets
    // targets_thread Data Structure: Arc<RwLock<vec<target_str>>>
    // RwLock is used for sharing read-only data without lock among threads.

    // let mut targets:Vec<&str> = Vec::new();
    // let output = Command::new("rustup").arg("target")
    //                                   .arg("list")
    //                                   .output().expect("Can't get rust targets!").stdout;
    // let output_str = String::from_utf8_lossy(&output);
    // let mut lines = output_str.lines();
    // while let Some(line) = lines.next() {
    //     targets.push(line.clone());
    // }
    
    // let targets_thread = Arc::new(RwLock::new(targets));
    // info!("Rust Targets: {:?}", targets_thread.read().unwrap());

    // Prepare `Accuracy Evaluation` crates DB
    let conn = Arc::new(Mutex::new(
        Client::connect(
            "host=localhost dbname=crates user=postgres password=postgres",
            NoTls,
        )
        .unwrap(),
    ));
    let unevaluated_crates = find_unevaluated_crates(Arc::clone(&conn));
    let workers = THREADNUM;

    let mb = Arc::new(MultiBar::new());
    let mut mpb = mb.create_bar(unevaluated_crates.len() as u64);
    mpb.format("╢▌▌░╟");
    mpb.set(0);

    let (tx, rx) = channel::bounded(2 * workers);

    let mut handles = vec![];
    for i in 0..workers {
        let rx = rx.clone();
        let conn = Arc::clone(&conn);
        let mb = Arc::clone(&mb);
        // let targets_thread = Arc::clone(&targets_thread);

        // Thread Operation
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
                    .download_folder(Path::new(CRATESDIR))
                    .parallel_requests(1)
                    .build()
                    .expect("Fatal Error, build downloader fails!");

                while let Ok(crate_info) = rx.recv(){
                    let crate_info:CrateInfo = crate_info;
                    pb.set(0);
                    pb.message(&(crate_info.name));

                    if let Err(e) = fetch_crate(&mut downloader, &crate_info.name, &crate_info.version_num) {
                        warn!("Thread {}: Fetch fails: crate {:?}, {}", i, crate_info, e);
                        store_fails_info(Arc::clone(&conn), crate_info.crate_id)
                    } else {
                        pb.inc();
                        if let Err(e) = deal_crate(Arc::clone(&conn), &crate_info) {
                            warn!("Thread {}: Deal fails: crate {:?}, {}", i, crate_info, e);
                            store_fails_info(Arc::clone(&conn), crate_info.crate_id)
                        } else {
                            pb.inc();
                            info!("Thread {}: Done crates - {}", i, crate_info.crate_id);
                        }
                    }
                    remove_dir_all(&format!("{}/{}", CRATESDIR, crate_info.name)).unwrap_or_default();
                }

                pb.finish();
            })
            .unwrap_or_default();
            panic::set_hook(old_hook);
        }));
    }

    handles.push(thread::spawn(move || mb.listen()));
    // Send data to child thread
    for crate_info in unevaluated_crates {
        tx.send(crate_info).expect("Fatal error, send fails");
        mpb.inc();
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

fn deal_crate(
    conn: Arc<Mutex<Client>>,
    crate_info: &CrateInfo,
) -> Result<()> {
    // Get resolution results of both pipeline and cargo tree
    // Extract each dependency, deduplicate and store in csv format
    let cargotree_crates = cargo_tree_resolution(crate_info);
    let path_string = format!("{}/{}-{}.csv", CARGOTREE_DEPENDENCYDIR, crate_info.name, crate_info.version_num);
    write_dependency_file(path_string, &cargotree_crates);

    // Update resolved status
    let query = format!(
        "UPDATE accuracy_evaluation_status SET status = 'resolved' WHERE crate_id = {}"
        , crate_info.crate_id
    );
    conn.lock().unwrap().query(&query, &[])?;
    Ok(())
}






fn cargo_tree_resolution(
    crate_info: &CrateInfo
) -> HashMap<String, HashSet<String>>{
    let name = &crate_info.name;
    let version = &crate_info.version_num;

    // Decompress
    let output = Command::new("tar").arg("-zxf")
                        .arg(format!("{}/{}/{}.tgz", CRATESDIR, name, version))
                        .arg("-C")
                        .arg(format!("{}/{}", CRATESDIR, name))
                        .output().expect("Ungzip exec error!");
    let output_str = String::from_utf8_lossy(&output.stdout);
    info!("Decompress {}/{}/{}.tgz success: {}", CRATESDIR, name, version, output_str);
    
    // Cargo Dependency Resolution (Remove lock file to keep it up-to-date)
    let output = Command::new("rm").arg(format!("{}/{}/{}-{}/Cargo.lock", CRATESDIR, name, name, version))
                                    .output().expect("rm Cargo.lock exec error!");
    let output_str = String::from_utf8_lossy(&output.stdout);
    info!("Remove Cargo.lock {}/{}/{}-{}/Cargo.toml success: {}", CRATESDIR, name, name, version, output_str);
    
    // Run `cargo tree` in each target, get union of their dependency graph as final results.
    // Data structure of `dependencies`: HashMap<crate_name, HashSet<versions> >
    let mut dependencies:HashMap<String, HashSet<String>> = HashMap::new();
    let re = Regex::new(r"[\w-]+ v[0-9]+.[0-9]+.[0-9]+[\S]*").unwrap();
    let output = Command::new("cargo").arg("tree")
                                        .arg("--manifest-path")
                                        .arg(format!("{}/{}/{}-{}/Cargo.toml", CRATESDIR, name, name, version)) // toml path
                                        .arg("-e")
                                        .arg("no-dev")
                                        .arg("--all-features")
                                        .arg("--target")
                                        .arg("all")
                                        .output().expect("tree exec error!");
    let output_str = String::from_utf8_lossy(&output.stdout);
    for cap in re.captures_iter(&output_str) {
        let dep = &cap[0];
        let name_ver:Vec<&str> = dep.split(' ').collect();
        let dep_name = String::from(name_ver[0]);
        let mut dep_ver = String::from(name_ver[1]);
        dep_ver.remove(0); // Remove char 'v' at the beginning of dep_ver
        let crate_name = dependencies.entry(dep_name).or_insert(HashSet::new());
        (*crate_name).insert(dep_ver);
    }
    let crate_name = dependencies.entry(String::from(name)).or_insert(HashSet::new());
    (*crate_name).remove(&String::from(version)); // Remove current version
    dependencies
    
}





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
