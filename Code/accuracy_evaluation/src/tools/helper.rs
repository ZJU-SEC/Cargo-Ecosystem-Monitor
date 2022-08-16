use std::collections::{HashSet, HashMap};
use std::fs::{create_dir, File, };
use std::path::Path;
use std::io::BufReader;
use std::io::prelude::*;

use anyhow::{anyhow, Result};
use downloader::{Download, Downloader};


pub const CRATESDIR:&str = "./on_process";
pub const OUTPUTDIR:&str = "./output";
pub const RESULTSDIR:&str = "./output/results";
pub const RAW_DEPENDENCYDIR:&str = "./output/dependency";
pub const CARGOTREE_DEPENDENCYDIR:&str = "./output/dependency/cargotree";
pub const PIPELINE_DEPENDENCYDIR:&str = "./output/dependency/pipeline";

// Write `dependencies` to `file` in csv format.
pub fn write_dependency_file(path_string: String, dependencies: &HashMap<String, HashSet<String>>){
    let path = Path::new(path_string.as_str());
    let display = path.display();   
    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't create {}: {}", display, why),
        Ok(file) => file,
    };
    for(crate_name, versions) in dependencies {
        for version in versions {
            let dep_name = format!("{},", crate_name);
            let dep_ver = format!("{}", version);
            let line = dep_name+ &dep_ver + "\n";
            if let Err(why) = file.write_all(line.as_bytes()) {
                panic!("couldn't write to {}: {}", path.display(), why);
            }
        }
    }
}


// Write `dependencies` to `file` in csv format, sorted.
pub fn write_dependency_file_sorted(path_string: String, dependencies: &HashMap<String, HashSet<String>>){
    let mut content:Vec<String> = Vec::new();
    let path = Path::new(path_string.as_str());
    let display = path.display();   
    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't create {}: {}", display, why),
        Ok(file) => file,
    };
    for(crate_name, versions) in dependencies {
        for version in versions {
            let dep_name = format!("{},", crate_name);
            let dep_ver = format!("{}", version);
            let line = dep_name+ &dep_ver + "\n";
            content.push(line);
        }
    }
    content.sort();
    for line in content {
        if let Err(why) = file.write_all(line.as_bytes()) {
            panic!("couldn't write to {}: {}", path.display(), why);
        }
    }
}


pub fn get_dependency_num(dependencies: &HashMap<String, HashSet<String>>) -> usize {
    let mut num = 0;
    for (_, versions) in dependencies {
        num += versions.len();
    }
    num
}

pub fn from_dependency_file(path_string: String) -> Result<HashMap<String, HashSet<String>>>{
    let mut dependencies = HashMap::new();
    let path = Path::new(path_string.as_str());
    let display = path.display();   
    let file = match File::open(&path) {
        Err(why) => panic!("couldn't open {}: {}", display, why),
        Ok(file) => file,
    };

    // Read dependency file
    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    buf_reader.read_to_string(&mut contents)?;

    for line in contents.lines(){
        let name_ver:Vec<&str> = line.split(',').collect();
        let dep_name = String::from(name_ver[0]);
        let dep_ver = String::from(name_ver[1]);
        let crate_name = dependencies.entry(dep_name).or_insert(HashSet::new());
        (*crate_name).insert(dep_ver);
    }
    
    Ok(dependencies)
}

pub fn fetch_crate(
    downloader: &mut Downloader,
    name: &str,
    version: &str,
) -> Result<()> {
    let mut dls = vec![];

    create_dir(Path::new(&format!("{}/{}", CRATESDIR, name))).unwrap_or_default();

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

    return Ok(());
}