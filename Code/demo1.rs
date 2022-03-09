use cargo::core::registry::PackageRegistry;
use cargo::core::resolver::{CliFeatures, HasDevUnits};
use cargo::core::Workspace;
use cargo::ops;
use cargo::util::Config;
use std::path::Path;

fn main() {
    let config = Config::default().unwrap();
    let ws = Workspace::new(
        &Path::new("path/to/toml files"),
        &config,
    )
    .unwrap();

    let mut registry = PackageRegistry::new(ws.config()).unwrap();
    let mut resolve = ops::resolve_with_previous(
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

    print!("{:#?}", resolve.query("semver"));
}