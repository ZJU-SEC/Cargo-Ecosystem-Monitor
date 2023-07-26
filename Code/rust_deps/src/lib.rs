use cargo::core::dependency::{DepKind, self};
use cargo::core::summary::{FeatureValue, self};
use cargo::core::registry::PackageRegistry;
use cargo::core::resolver::{CliFeatures, ForceAllTargets, HasDevUnits, ResolveOpts, Resolve};
use cargo::core::{PackageIdSpec, Workspace, Shell, Package, PackageId, Summary, features};
use cargo::util::{Config, graph};
use cargo::ops::{self, tree::TreeOptions, Packages};

use std::collections::{HashMap, HashSet, VecDeque};
use std::env::{self, current_dir};
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::result::Result::Ok;

use anyhow::{Context, Result};


/// Resolve version's dependencies.
/// Is is recommended to be used for small mount of queries as it lacks of performance optimization. 
pub fn resolve_deps_of_version_once(
    name: String,
    num: String,
) -> Result<String> {
    let mut features = Vec::new();

    // Create virtual env by creating toml file
    // Fill toml contents
    let current_path = current_dir()?;
    let dep_filename = format!("dep_once.toml");
    let current_toml_path = format!("{}/{}", current_path.display(), dep_filename);

    // Create toml file
    let file = format_virt_toml_file(&name, &num, &features);
    File::create(&current_toml_path)?
        .write_all(file.as_bytes())
        .expect("Write failed");

    // 1. Pre Resolve: To find all features of given crate
    // Create virtual env by setting correct workspace
    let config = Config::new(
        Shell::new(),
        env::current_dir()?,
        format!("{}/job_once", current_path.to_str().unwrap()).into(),
    );
    let ws = Workspace::new(&Path::new(&current_toml_path), &config)?;
    let mut registry = PackageRegistry::new(ws.config())?;
    let resolve = ops::resolve_with_previous(
        &mut registry,
        &ws,
        &CliFeatures::new_all(true),
        HasDevUnits::No,
        None,
        None,
        &[],
        true,
    )?;

    // Find all `features` including user-defined and optional dependency
    if let Ok(res) = resolve.query(&format!("{}:{}", name, num)) {
        for feature in resolve.summary(res).features().keys() {
            features.push(feature.as_str());
        }
    } else {
        println!("Resolve {}-{} fails to find any features.", name, num);
    }
    // println!("All Features: {:?}", features);

    // 2. Double resolve: This time resolve with features
    // The resolve result is the final one.
    let file = format_virt_toml_file(&name, &num, &features);
    // println!("file: {}", file);
    File::create(&current_toml_path)?
        .write_all(file.as_bytes())
        .expect("Write failed");
    let config = Config::new(
        Shell::new(),
        env::current_dir()?,
        format!("{}/job_once", current_path.to_str().unwrap()).into(),
    );
    let ws = Workspace::new(&Path::new(&current_toml_path), &config).unwrap();
    let mut registry = PackageRegistry::new(ws.config()).unwrap();
    let resolve = ops::resolve_with_previous(
        &mut registry,
        &ws,
        &CliFeatures::new_all(true),
        // &features,
        HasDevUnits::No,
        None,
        None,
        &[],
        // &[PackageIdSpec::parse("dep").unwrap()],
        true,
    )
    .unwrap();

    // 3. Start Formatting Resolve and store into DB
    // Resolve the dep tree.
    let mut set = HashSet::new();
    let root = resolve.query(&name)?;
    let mut v = VecDeque::new();
    let mut level = 1;
    v.extend([Some(root), None]);

    while let Some(next) = v.pop_front() {
        if let Some(pkg) = next {
            for (pkg, _) in resolve.deps(pkg) {
                set.insert((
                    (pkg.name().to_string(), pkg.version().to_string()),
                    level,
                ));
                v.push_back(Some(pkg));
            }
        } else {
            level += 1;
            if !v.is_empty() {
                v.push_back(None)
            }
        }
    }

    // Format output
    let mut deps = String::new();
    for (version_to, level) in set {
        deps.push_str(&format!("{},{},{}\n", version_to.0, version_to.1, level));
    }

    Ok(deps)
}

/// Resolve version's dependencies. This time, we print raw dependency results with full info.
/// Is is recommended to be used for small mount of queries as it lacks of performance optimization. 
pub fn resolve_deps_of_version_once_full(
    name: String,
    num: String,
) -> Result<String> {
    let mut features = Vec::new();

    // Create virtual env by creating toml file
    // Fill toml contents
    let current_path = current_dir()?;
    let dep_filename = format!("dep_once.toml");
    let current_toml_path = format!("{}/{}", current_path.display(), dep_filename);

    // Create toml file
    let file = format_virt_toml_file(&name, &num, &features);
    File::create(&current_toml_path)?
        .write_all(file.as_bytes())
        .expect("Write failed");

    // 1. Pre Resolve: To find all features of given crate
    // Create virtual env by setting correct workspace
    let config = Config::new(
        Shell::new(),
        env::current_dir()?,
        format!("{}/job_once", current_path.to_str().unwrap()).into(),
    );
    let ws = Workspace::new(&Path::new(&current_toml_path), &config)?;
    let mut registry = PackageRegistry::new(ws.config())?;
    let resolve = ops::resolve_with_previous(
        &mut registry,
        &ws,
        &CliFeatures::new_all(true),
        HasDevUnits::No,
        None,
        None,
        &[],
        true,
    )?;

    // Find all `features` including user-defined and optional dependency
    if let Ok(res) = resolve.query(&format!("{}:{}", name, num)) {
        for feature in resolve.summary(res).features().keys() {
            features.push(feature.as_str());
        }
    } else {
        println!("Resolve {}-{} fails to find any features.", name, num);
    }
    // println!("All Features: {:?}", features);

    // 2. Double resolve: This time resolve with features
    // The resolve result is the final one.
    let file = format_virt_toml_file(&name, &num, &features);
    // println!("file: {}", file);
    File::create(&current_toml_path)?
        .write_all(file.as_bytes())
        .expect("Write failed");
    let config = Config::new(
        Shell::new(),
        env::current_dir()?,
        format!("{}/job_once", current_path.to_str().unwrap()).into(),
    );
    let ws = Workspace::new(&Path::new(&current_toml_path), &config).unwrap();
    let mut registry = PackageRegistry::new(ws.config()).unwrap();
    let resolve = ops::resolve_with_previous(
        &mut registry,
        &ws,
        &CliFeatures::new_all(true),
        // &features,
        HasDevUnits::No,
        None,
        None,
        &[],
        // &[PackageIdSpec::parse("dep").unwrap()],
        true,
    )
    .unwrap();

    Ok(format!("{:?}", resolve))
}

fn format_virt_toml_file(name: &String, version_num: &String, features: &Vec<&str>) -> String {
    let mut file = String::from(
        r#"[package]
name = "dep"
version = "0.1.0"
edition = "2021"
[dependencies]"#,
    );
    file.push('\n');

    // Add all features
    file.push_str(&format!(
        "{} = {}version = \"={}\", features = [",
        name, "{", version_num
    ));
    for feature in features {
        file.push_str(&format!("\"{}\",", feature));
    }
    file.push_str("]}");
    file
}


pub fn test_registry(
    name: String,
    num: String,
) -> Result<String> {
    let mut features = Vec::new();

    // Create virtual env by creating toml file
    // Fill toml contents
    let current_path = current_dir()?;
    let dep_filename = format!("dep_once.toml");
    let current_toml_path = format!("{}/{}", current_path.display(), dep_filename);

    // Create toml file
    let file = format_virt_toml_file(&name, &num, &features);
    File::create(&current_toml_path)?
        .write_all(file.as_bytes())
        .expect("Write failed");

    // 1. Pre Resolve: To find all features of given crate
    // Create virtual env by setting correct workspace
    let config = Config::new(
        Shell::new(),
        env::current_dir()?,
        format!("{}/job_once", current_path.to_str().unwrap()).into(),
    );
    let ws = Workspace::new(&Path::new(&current_toml_path), &config).unwrap();
    let mut registry = PackageRegistry::new(ws.config()).unwrap();
    let resolve = ops::resolve_with_previous(
        &mut registry,
        &ws,
        &CliFeatures::new_all(true),
        HasDevUnits::No,
        None,
        None,
        &[],
        true,
    ).unwrap();



    // Find all `features` including user-defined and optional dependency
    if let Ok(res) = resolve.query(&format!("{}:{}", name, num)) {
        for feature in resolve.summary(res).features().keys() {
            features.push(feature.as_str());
        }
    } else {
        println!("Resolve {}-{} fails to find any features.", name, num);
    }
    // println!("All Features: {:?}", features);

    // 2. Double resolve: This time resolve with features
    // The resolve result is the final one.
    let file = format_virt_toml_file(&name, &num, &features);
    // println!("file: {}", file);
    File::create(&current_toml_path)?
        .write_all(file.as_bytes())
        .expect("Write failed");
    let config = Config::new(
        Shell::new(),
        env::current_dir()?,
        format!("{}/job_once", current_path.to_str().unwrap()).into(),
    );
    let ws = Workspace::new(&Path::new(&current_toml_path), &config).unwrap();
    let mut registry = PackageRegistry::new(ws.config()).unwrap();
    let resolve = ops::resolve_with_previous(
        &mut registry,
        &ws,
        &CliFeatures::new_all(true),
        // &features,
        HasDevUnits::No,
        None,
        None,
        &[],
        // &[PackageIdSpec::parse("dep").unwrap()],
        true,
    )
    .unwrap();

    // 3. Formatting Resolve and stripe 
    // Resolve the dep tree.

    // println!("{}", format!("Dep: {:?}", resolve));
    // if let Ok(res) = resolve.query(&format!("{}:{}", name, num)) {
    //     println!("Features:{:#?}", resolve.summary(res).features());
    // } else {
    //     println!("NO RES");
    // }

    let graph = Graph::build_deps(&name, resolve);
    graph.print();
    Ok(String::new())
}


#[derive(Hash, Eq, PartialEq, Clone)]
struct Version {
    name: String,
    num: String,
}


impl Version {
    fn new(name: String, num: String) -> Version {
        Version { name, num }
    }

    fn to_string(&self) -> String {
        format!("{} v{}", self.name, self.num)
    }
}

pub struct Deps {
    from: Version,
    to: Vec<Version>,
}

impl Deps {
    fn new(name: String, num: String) -> Deps {
        Deps { from: Version::new(name, num), to: Vec::new() }
    }

    fn get_dep_from(&self) -> Version{
        self.from.clone()
    }

    fn push_dep(&mut self, name: String, num: String) {
        self.to.push(Version::new(name, num))
    }

}

pub struct Graph {
    versions_deps: HashMap<Version, Vec<Version>>
}

impl Graph {
    pub fn new() -> Graph {
        Graph { versions_deps: HashMap::new() }
    }


    pub fn push(&mut self, deps: Deps){
        self.versions_deps.insert(deps.from, deps.to);
    }

    /// Given the `name` of the root package and its `resolve`,
    /// we'll build all its "real" dependencies.
    /// This removes fake dependencies, especially with "dep?/feature" format
    /// but acutally not dependending on "dep".
    pub fn build_deps(name:&str, resolve: Resolve) -> Graph {
        // println!("Resolve: {:?}", resolve);
        let mut graph = Graph::new();
        let root = resolve.query(name).unwrap();
        let mut v = VecDeque::new();
        let mut level = 1;
        v.extend([Some(root), None]);


        for pkg in resolve.sort(){
            let mut deps = Deps::new(pkg.name().to_string()
                                        , pkg.version().to_string());
            let enabled_features = resolve.features(pkg);
            let mut enabled_dep = Vec::new();
            let summary = resolve.summary(pkg);
            let mut suspect_deps = HashSet::new();
            // Find out dependencies enabled through features "dep:pkg"
            for (feature, feature_deps) in summary.features() {
                if !enabled_features.contains(feature) {
                    continue;
                }
                for fdep in feature_deps {
                    if let FeatureValue::Dep {
                        dep_name} = fdep {
                            enabled_dep.push(dep_name);
                        }
                }
            }
            // Search for deps with "dep?/feature" feature requirements
            for (_, feature_deps) in summary.features() {
                for fdep in feature_deps {
                    if let FeatureValue::DepFeature {
                        dep_name,
                        dep_feature: _,
                        weak } = fdep {
                            if *weak == true && !enabled_features.contains(dep_name)
                                && !enabled_dep.contains(&dep_name){
                                suspect_deps.insert(dep_name);
                            }
                        }
                }
            }
            // Make sure that the deps are optional and non-depvelopment
            // for rdep in summary.dependencies() {
            //     let name = &rdep.package_name();
            //     if !suspect_deps.contains(name){
            //         continue;
            //     }
            //     if rdep.kind() != DepKind::Development &&
            //         rdep.is_optional() == true{
            //             suspect_deps.remove(name);
            //         }
            // }
            // if !suspect_deps.is_empty(){
            //     println!("Suspect deps of {}: {:?}", pkg.to_string(), suspect_deps);
            // }
            // By default, we add all dependencies in the `resolve`.
            // However, for dependencies from "dep?/feature", we need to check whether "dep" is really used.
            // If not, we remove the "dep".
            for (pkg, _) in resolve.deps(pkg) {
                let from = deps.get_dep_from();
                // map.entry(from).and_modify(|count| *count += 1 ).or_insert(1 as usize);
                
                if suspect_deps.contains(&pkg.name()) {
                    continue;
                }
                deps.push_dep(pkg.name().to_string(), pkg.version().to_string());
                v.push_back(Some(pkg));
            }
            graph.push(deps);
        }
        // Although we have removed fake dependency "pkg" -> "dep",
        // the package "dep" should be moved too if no packages dependend on it after "fake removal.
        let mut dep_count = HashMap::new();
        for (from, to) in &graph.versions_deps {
            dep_count.insert(from.clone(), 0);
            for v in to {
                dep_count.entry(v.clone()).and_modify(|cnt| *cnt += 1).or_insert(1);
            }
        }
        loop{
            // Find unused pkgs
            let mut pkgs = HashSet::new();
            let mut real_pkgs = HashSet::new();
            real_pkgs.insert(Version::new("dep".to_string(), "0.1.0".to_string())); // our virtual pack
            for pkg in graph.versions_deps.keys() {
                pkgs.insert(pkg.clone());
            }
            for (_, deps) in &graph.versions_deps {
                for dep in deps {
                    real_pkgs.insert(dep.clone());
                }
            }
            // If all packages have packages depending on it, then exit.
            if pkgs.len() == real_pkgs.len(){
                break;
            }
            // Or we stripe useless packages
            for pkg in &pkgs {
                if !real_pkgs.contains(pkg){
                    println!("Remove: {}", pkg.name);
                    graph.versions_deps.remove(pkg);
                }
            }
        }
        graph
    }

    pub fn print(&self){
        println!("Graph {{");

        for (from, to) in &self.versions_deps {
            println!("  - {}", from.to_string());
            for dep in to {
                println!("    - {}", dep.to_string());
            }
        }

        println!("}}");
    }
}