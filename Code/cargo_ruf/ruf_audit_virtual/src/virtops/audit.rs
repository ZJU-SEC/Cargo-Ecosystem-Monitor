use std::io::Write;

use cargo_lock::dependency::graph::NodeIndex;
use fxhash::{FxHashMap, FxHashSet};
use petgraph::visit;
use semver::Version;

use super::ops::DepOpsVirt;
use crate::core::{AuditError, DepTreeManager};

/// The main audit function.
/// The debugger receives an output stream to write debug information.
pub fn audit(
    name: &str,
    ver: &str,
    workspace: &str,
    debugger: &mut impl Write,
) -> Result<u32, AuditError> {
    // Init a tree first
    let ops = DepOpsVirt::new(name, ver, workspace)?;
    let mut deptree = DepTreeManager::new(ops, 63)?;
    let logical_root = deptree
        .get_graph()
        .neighbors(deptree.get_root())
        .next()
        .unwrap();
    deptree.set_local(&logical_root);

    // Check issues and fix them.
    check_fix(deptree, debugger)
}

fn check_fix(
    mut deptree: DepTreeManager<DepOpsVirt>,
    debugger: &mut impl Write,
) -> Result<u32, AuditError> {
    for rustc in (0..64).rev() {
        deptree.update_rustv(rustc);
        writeln!(
            debugger,
            "[VirtAudit Debug] check_fix: Checking rustc version {}.",
            rustc,
        )
        .unwrap();
        let issue_deps = check_issue(&deptree, debugger)?;
        if issue_deps.is_empty() {
            writeln!(
                debugger,
                "[VirtAudit Debug] check_fix: Rustc version {} has no issues.",
                rustc
            )
            .unwrap();
            return Ok(rustc);
        }

        let first_issue = issue_deps.first().unwrap().to_owned();
        if let Ok(fixes) = check_fixable(&mut deptree, issue_deps, debugger) {
            if let Err(e) = try_fix(
                &mut deptree,
                first_issue,
                fixes.into_iter().next().unwrap(),
                debugger,
            ) {
                writeln!(debugger,
                "[VirtAudit Debug] check_fix: fix failure for rustc version {} with error: {:?}", rustc, e).unwrap();
            } else {
                writeln!(
                    debugger,
                    "[VirtAudit Debug] check_fix: Rustc version {} issues fixed.",
                    rustc,
                )
                .unwrap();
                return Ok(rustc);
            }
        } else {
            writeln!(
                debugger,
                "[VirtAudit Debug] check_fix: Rustc version {} got issues cannot be fixed.",
                rustc
            )
            .unwrap();
        }
    }

    Err(AuditError::FunctionError(None, None))
}

fn check_issue(
    deptree: &DepTreeManager<DepOpsVirt>,
    debugger: &mut impl Write,
) -> Result<Vec<NodeIndex>, AuditError> {
    // Extract current used rufs.
    let used_rufs = deptree.extract_rufs()?;

    // We do bfs and thus fix problems up to down.
    let graph = deptree.get_graph();
    // In virt audit, real root is the child of `root`.
    let root = graph.neighbors(deptree.get_root()).next().unwrap();

    // Collect all ruf issues first.
    let mut issue_deps = Vec::new();
    let mut bfs = visit::Bfs::new(&graph, root);
    while let Some(nx) = bfs.next(&graph) {
        let node = &graph[nx];
        let name_ver = format!("{}@{}", node.name, node.version);
        if let Some(rufs) = used_rufs.get(&name_ver) {
            let issue_rufs = deptree.filter_rufs(rufs.iter().collect());
            if !issue_rufs.is_empty() {
                // Ok here we got issues
                writeln!(
                    debugger,
                    "[VirtAudit Debug] check_issue: Found issue package {}@{} rufs: {:?}",
                    node.name, node.version, issue_rufs
                )
                .unwrap();
                issue_deps.push(nx);
            }
        }
    }

    Ok(issue_deps)
}

fn check_fixable(
    deptree: &mut DepTreeManager<DepOpsVirt>,
    issue_deps: Vec<NodeIndex>,
    debugger: &mut impl Write,
) -> Result<Vec<FxHashMap<NodeIndex, Version>>, AuditError> {
    let graph = deptree.get_graph();
    let mut fixes = Vec::new();

    // Check possible fix for each issue.
    for nx in issue_deps {
        match deptree.issue_fixable(nx, debugger) {
            Ok(fix) => {
                writeln!(
                    debugger,
                    "[VirtAudit Debug] check_fixable: Issue dep {}@{} is fixable.",
                    graph[nx].name, graph[nx].version,
                )
                .unwrap();
                fixes.push(fix);
            }
            Err(e) => {
                writeln!(
                    debugger,
                    "[VirtAudit Debug] check_fixable: Issue dep {}@{} is not fixable with error {:?}.",
                    graph[nx].name, graph[nx].version, e
                )
                .unwrap();
                return Err(e);
            }
        }
    }

    Ok(fixes)
}

fn try_fix(
    deptree: &mut DepTreeManager<DepOpsVirt>,
    first_issue: NodeIndex,
    first_fix: FxHashMap<NodeIndex, Version>,
    debugger: &mut impl Write,
) -> Result<(), AuditError> {
    // For loop detect.
    let mut already_fixed = FxHashSet::default();

    {
        let graph = deptree.get_graph();
        let mut fix = Vec::new();

        for (nx, fix_ver) in first_fix {
            fix.push((
                graph[nx].name.to_string(),
                graph[nx].version.to_string(),
                fix_ver.to_string(),
            ));
        }

        writeln!(
            debugger,
            "[VirtAudit Debug] try_fix: Try fix {} with {:?}",
            graph[first_issue].name,
            fix.iter()
                .map(|(name, prev_ver, fix_ver)| format!("{}@{} -> {}", name, prev_ver, fix_ver))
                .collect::<Vec<String>>()
        )
        .unwrap();

        // Apply the first fix.
        deptree.update_pkgs(fix, debugger)?;

        already_fixed.insert(first_issue);
    }

    // Since the tree has changed, the following fixes may not be valid, so we have to resolve the fix again.
    loop {
        let graph = deptree.get_graph();

        let issue_dep = check_issue(deptree, debugger)?.first().cloned();
        if issue_dep.is_none() {
            return Ok(());
        }

        let issue_dep = issue_dep.unwrap();
        let fix = deptree.issue_fixable(issue_dep, debugger)?;
        assert!(!fix.is_empty(), "Fatal, no fix found when fixing issue.");

        let fix: Vec<(String, String, String)> = fix
            .into_iter()
            .map(|(nx, fix_ver)| {
                (
                    graph[nx].name.to_string(),
                    graph[nx].version.to_string(),
                    fix_ver.to_string(),
                )
            })
            .collect();

        writeln!(
            debugger,
            "[VirtAudit Debug] try_fix: Try fix {} with {:?}",
            graph[issue_dep].name,
            fix.iter()
                .map(|(name, prev_ver, fix_ver)| format!("{}@{} -> {}", name, prev_ver, fix_ver))
                .collect::<Vec<String>>()
        )
        .unwrap();

        deptree.update_pkgs(fix, debugger)?;

        let check_loop = already_fixed.insert(issue_dep);

        if !check_loop {
            return Err(AuditError::FunctionError(
                Some(format!("Dupfix, maybe a loop happens",)),
                Some(issue_dep),
            ));
        }
    }
}

#[test]
fn test_audit() {
    use std::sync::{Arc, Mutex};

    const WORKSPACE_PATH: &str = "/home/ubuntu/Workspaces/Cargo-Ecosystem-Monitor/Code/cargo_ruf/ruf_audit_virtual/virt_work";
    let stdout = Arc::new(Mutex::new(std::io::stdout()));
    let mut buffer = stdout.lock().unwrap();

    // let res = audit("taxonomy", "0.3.1", WORKSPACE_PATH, &mut *buffer);
    // let res = audit("pyo3", "0.9.2", WORKSPACE_PATH, &mut *buffer);

    // let res = audit(
    //     "rustc-ap-rustc_errors",
    //     "12.0.0",
    //     WORKSPACE_PATH,
    //     &mut *buffer,
    // );

    // let res = audit("tar", "0.4.0", WORKSPACE_PATH, &mut *buffer);
    // let res = audit("chrono-tz", "0.1.0", WORKSPACE_PATH, &mut *buffer);
    // let res = audit("leaf", "0.0.1", WORKSPACE_PATH, &mut *buffer);

    // We lack rustc-ap-rustc_expand infos.
    // let res = audit("kunai", "0.3.0", WORKSPACE_PATH, &mut *buffer);
    // let res = audit("hsr-codegen", "0.2.0", WORKSPACE_PATH, &mut *buffer);
    // let res = audit("deuterium_orm", "0.5.0", WORKSPACE_PATH, &mut *buffer);
    let res = audit("tari_comms_dht", "0.8.1", WORKSPACE_PATH, &mut *buffer);

    println!("RESULTS: {:?}", res);
}
