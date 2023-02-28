use cargo::core::registry::PackageRegistry;
use cargo::core::resolver::{CliFeatures, HasDevUnits};
use cargo::core::Workspace;
use cargo::ops;
use cargo::util::Config;
use std::path::Path;
use std::env::{current_dir};
use std::time::SystemTime;

fn main() {
    // Get absolute current toml file path.
    // You can change the value of `current_toml_path` to analyse other toml file
    let current_path = current_dir().unwrap();
    let mut  current_toml_path = String::new();
    current_toml_path.push_str(current_path.to_str().unwrap());
    current_toml_path.push_str("/Cargo.toml");
    println!("Cargo.toml path:{}",current_toml_path.as_str());

    let config = Config::default().unwrap();
    let ws = Workspace::new(
        &Path::new(current_toml_path.as_str()),
        &config,
    )
    .unwrap();

    let time_before = SystemTime::now();
    let mut registry = PackageRegistry::new(ws.config()).unwrap();
    let resolve = ops::resolve_with_previous(
        &mut registry,
        &ws,
        &CliFeatures::new_all(true),
        HasDevUnits::Yes,
        None,
        None,
        &[],
        true,
    )
    .unwrap();
    let time_after = SystemTime::now();
    let time_consume = time_after.duration_since(time_before);

    // `resolve` is the full dependency in toml file.
    // println!("{:#?}", resolve);
    println!("{:#?}", resolve.query("semver"));// Get specific crate version in the dependency
    println!("Parse time consumption: {:?}", time_consume.unwrap())
}