use std::collections::{HashMap, HashSet};
use std::env;
use std::path::Path;

use cargo::core::compiler::{CompileKind, RustcTargetData};
use cargo::core::dependency::DepKind;
use cargo::core::registry::PackageRegistry;
use cargo::core::resolver::{CliFeatures, ForceAllTargets, HasDevUnits};
use cargo::core::{PackageIdSpec, Workspace, Shell, Package, PackageId};
use cargo::ops::tree::{Target, EdgeKind, Prefix, Charset};
use cargo::ops::{self, tree::TreeOptions, Packages};
use cargo::util::Config;

use anyhow::Result;

mod graph;

const DEMO: &str = "/Users/wyffeiwhe/Desktop/Research/Supplychain/Cargo-Ecosystem-Monitor/Code/demo/feature_level_dependency";
const NAME: &str = "feature_level_dependency";

fn main() {
    // println!("{:?}", bad_resolve(DEMO));
    println!("{:?}", fix_resolve(DEMO, NAME));
}

fn bad_resolve(path: &str) -> Result<()> {
    let config = Config::new(
        Shell::new(),
        env::current_dir()?,
        path.into(),
    );
    let ws = Workspace::new(&Path::new(&format!("{path}/Cargo.toml")), &config)?;
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

    println!("{:?}", resolve);

    Ok(())
}

fn fix_resolve(path: &str, name: &str) -> Result<()> {
    let config = Config::default()?;

    let ws = Workspace::new(&Path::new(&format!("{path}/Cargo.toml")), &config)?;
    let requested_targets = Vec::new();
    let requested_kinds = CompileKind::from_requested_targets(ws.config(), &requested_targets)?;
    let target_data = RustcTargetData::new(&ws, &requested_kinds)?;

    let specs = PackageIdSpec::query_str(name, ws.members().map(|pkg| pkg.package_id()))?;
    let specs = [PackageIdSpec::from_package_id(specs)];

    let ws_resolve = ops::resolve_ws_with_opts(
        &ws,
        &target_data,
        &requested_kinds,
        &CliFeatures::new_all(false),
        &specs,
        HasDevUnits::Yes,
        ForceAllTargets::No,
    )?;

    let package_map: HashMap<PackageId, &Package> = ws_resolve
        .pkg_set
        .packages()
        .map(|pkg| (pkg.package_id(), pkg))
        .collect();

    // Default tree options
    let cli_features = CliFeatures::new_all(false);
    let packages = Packages::Default;
    let target = Target::Host;
    let mut edge_kinds = HashSet::new();
    edge_kinds.insert(EdgeKind::Dep(DepKind::Normal));
    edge_kinds.insert(EdgeKind::Dep(DepKind::Build));
    edge_kinds.insert(EdgeKind::Dep(DepKind::Development));
    let invert = vec![];
    let pkgs_to_prune = vec![];
    let prefix = Prefix::Indent;
    let no_dedupe = false;
    let duplicates = false;
    let charset = Charset::Utf8;
    let format = "{p}".to_string();
    let graph_features = false;
    let max_display_depth = u32::MAX;
    let no_proc_macro = false;

    let opts = TreeOptions {
        cli_features,
        packages,
        target,
        edge_kinds,
        invert,
        pkgs_to_prune,
        prefix,
        no_dedupe,
        duplicates,
        charset,
        format,
        graph_features,
        max_display_depth,
        no_proc_macro,
    };


    let mut g = graph::build(
        &ws,
        &ws_resolve.targeted_resolve,
        &ws_resolve.resolved_features,
        &specs,
        &CliFeatures::new_all(false),
        &target_data,
        &requested_kinds,
        package_map,
        &opts,
    )?;

    println!("{:?}", g.nodes);

    Ok(())
}
