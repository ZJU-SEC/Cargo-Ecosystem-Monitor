use std::collections::{HashSet, HashMap};
use std::fs::{create_dir, File, OpenOptions};
use std::panic::{self, catch_unwind};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::process::Command;
use std::io::prelude::*;

use simplelog::*;
use anyhow::{ Result};
use crossbeam::channel::{self};
use log::{error, info, warn};
use pbr::MultiBar;
use postgres::{Client, NoTls};
use regex::Regex;

use accuracy_evaluation::tools::db::*;
use accuracy_evaluation::tools::helper::*;


const THREADNUM:usize = 2; // Thread number

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
                .open("./pipeline_evaluation.log")
                .unwrap(),
        ),
    ])
    .unwrap();

    // Prepare work directories
    create_dir(Path::new(CRATESDIR)).unwrap_or_default(); // Crates file directory
    create_dir(Path::new(OUTPUTDIR)).unwrap_or_default();
    create_dir(Path::new(RESULTSDIR)).unwrap_or_default(); // Resolved results directory
    create_dir(Path::new(RAW_DEPENDENCYDIR)).unwrap_or_default(); // Resolved dependency directory

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
    let resolved_crates = find_resolved_crates(Arc::clone(&conn));
    let workers = THREADNUM;

    let mb = Arc::new(MultiBar::new());
    let mut mpb = mb.create_bar(resolved_crates.len() as u64);
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
            let mut targets:Vec<&str> = Vec::new();
            let output = Command::new("rustup").arg("target")
                                            .arg("list")
                                            .output().expect("Can't get rust targets!").stdout;
            let output_str = String::from_utf8_lossy(&output);
            let mut lines = output_str.lines();
            while let Some(line) = lines.next() {
                targets.push(line.clone());
            }

            let old_hook = panic::take_hook();
            panic::set_hook({
                Box::new(move |info| {
                    error!("Thread {}: panic, {}", i, info);
                })
            });

            catch_unwind(|| {
                let mut pb = mb.create_bar(2);
                while let Ok(crate_info) = rx.recv(){
                    let crate_info:CrateInfo = crate_info;
                    pb.set(0);
                    pb.message(&(crate_info.name));
                    pb.inc();

                    //////////////////////////////////////////////// 
                    // Main Process
                    ////////////////////////////////////////////////
                    if let Err(e) = deal_crate(Arc::clone(&conn), &targets, &crate_info) {
                        warn!("Thread {}: Deal fails: crate {:?}, {}", i, crate_info, e);
                        store_fails_info(Arc::clone(&conn), crate_info.crate_id)
                    } else {
                        pb.inc();
                        info!("Thread {}: Done crates - {}", i, crate_info.crate_id);
                    }
                }

                pb.finish();
            })
            .unwrap_or_default();
            panic::set_hook(old_hook);
        }));
    }

    handles.push(thread::spawn(move || mb.listen()));
    // Send data to child thread
    for crate_info in resolved_crates {
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
    targets: &Vec<&str>, 
    crate_info: &CrateInfo,
) -> Result<()> {
    // Get resolution results of both pipeline and cargo tree
    // Extract each dependency, deduplicate and store in csv format
    let path_string = format!("{}/{}-{}.csv", CARGOTREE_DEPENDENCYDIR, crate_info.name, crate_info.version_num);
    let cargotree_crates = from_dependency_file(path_string)?;

    // Same for pipeline
    let pipeline_crates = get_pipeline_results(Arc::clone(&conn), crate_info);
    let path_string = format!("{}/{}-{}.csv", PIPELINE_DEPENDENCYDIR, crate_info.name, crate_info.version_num);
    write_dependency_file(path_string, &pipeline_crates);

    // Compare two results
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

    // Write results
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
    let path_string = format!("{}/{}-{}.txt", RESULTSDIR, crate_info.name, crate_info.version_num);
    let path = Path::new(path_string.as_str());
    let display = path.display();   
    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't create {}: {}", display, why),
        Ok(file) => file,
    };
    if let Err(why) = file.write_all(line.as_bytes()) {
        panic!("couldn't write to {}: {}", path.display(), why);
    }

    // Update evaluated status
    let query = format!(
        "UPDATE accuracy_evaluation_status SET status = 'evaluated' WHERE crate_id = {}"
        , crate_info.crate_id
    );
    conn.lock().unwrap().query(&query, &[])?;
    Ok(())
}






fn cargo_tree_resolution(
    targets: &Vec<&str>, 
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
    for target in targets {
        let output = Command::new("cargo").arg("tree")
                                            .arg("--manifest-path")
                                            .arg(format!("{}/{}/{}-{}/Cargo.toml", CRATESDIR, name, name, version)) // toml path
                                            .arg("-e")
                                            .arg("no-dev")
                                            .arg("--all-features")
                                            .arg("--target")
                                            .arg(target)
                                            .output().expect("tree exec error!");
        let output_str = String::from_utf8_lossy(&output.stdout);
        for cap in re.captures_iter(&output_str) {
            let dep = &cap[0];
            let name_ver:Vec<&str> = dep.split(' ').collect();
            let dep_name = String::from(name_ver[0]);
            let dep_ver = name_ver[1].replace("v","");
            let crate_name = dependencies.entry(dep_name).or_insert(HashSet::new());
            (*crate_name).insert(dep_ver);
        }
    }
    let crate_name = dependencies.entry(String::from(name)).or_insert(HashSet::new());
    (*crate_name).remove(&String::from(version)); // Remove current version
    dependencies
    
}


