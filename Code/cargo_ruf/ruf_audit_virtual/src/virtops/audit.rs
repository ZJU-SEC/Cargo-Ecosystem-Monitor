use std::io::Write;

use cargo_lock::dependency::graph::NodeIndex;
use fxhash::FxHashMap;
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
) -> Result<(), AuditError> {
    // Init a tree first
    let ops = DepOpsVirt::new(name, ver, workspace)?;
    let deptree = DepTreeManager::new(ops, 63)?;
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
) -> Result<(), AuditError> {
    for rustc in (0..64).rev() {
        deptree.update_rustv(rustc);
        writeln!(
            debugger,
            "[VirtAudit Debug] check_fix: Checking rustc version {}",
            rustc
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
            return Ok(());
        }

        if let Ok(fixes) = check_fixable(&deptree, issue_deps, debugger) {
            if let Err(e) = try_fix(&mut deptree, fixes, debugger) {
                writeln!(debugger,
                "[VirtAudit Debug] check_fix: fix failure for rustc version {} with error: {:?}", rustc, e).unwrap();
            } else {
                writeln!(
                    debugger,
                    "[VirtAudit Debug] check_fix: Rustc version {} issues fixed, lockfile:\n{}",
                    rustc,
                    deptree.get_lockfile().unwrap()
                )
                .unwrap();
                return Ok(());
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
            writeln!(
                debugger,
                "[VirtAudit Debug] check_issue: Checking ruf enabled package {}@{} rufs: {:?}",
                node.name, node.version, rufs
            )
            .unwrap();
            if !deptree.filter_rufs(rufs.iter().collect()).is_empty() {
                // Ok here we got issues
                // issue_dep = Some((nx, node.name.to_string(), node.version.to_string()));
                issue_deps.push(nx);
            }
        }
    }

    Ok(issue_deps)
}

fn check_fixable(
    deptree: &DepTreeManager<DepOpsVirt>,
    issue_deps: Vec<NodeIndex>,
    debugger: &mut impl Write,
) -> Result<Vec<FxHashMap<NodeIndex, Version>>, AuditError> {
    let mut fixes = Vec::new();

    // Check possible fix for each issue.
    for nx in issue_deps {
        let res = deptree.issue_fixable(nx, debugger)?;
        fixes.push(res);
    }

    Ok(fixes)
}

fn try_fix(
    deptree: &mut DepTreeManager<DepOpsVirt>,
    fixes: Vec<FxHashMap<NodeIndex, Version>>,
    debugger: &mut impl Write,
) -> Result<(), AuditError> {
    {
        let graph = deptree.get_graph();

        let mut first_fix = Vec::new();
        for (nx, fix) in fixes.first().unwrap() {
            first_fix.push((
                graph[*nx].name.to_string(),
                graph[*nx].version.to_string(),
                fix.to_string(),
            ));
        }

        writeln!(
            debugger,
            "[VirtAudit Debug] try_fix: Try fix {} with {:?}",
            first_fix.last().unwrap().0,
            first_fix
                .iter()
                .map(|(name, prev_ver, fix_ver)| format!("{}@{} -> {}", name, prev_ver, fix_ver))
                .collect::<Vec<String>>()
        )
        .unwrap();

        // Apply the first fix.
        deptree.update_pkg(first_fix)?;
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

        let fixes: Vec<(String, String, String)> = fix
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
            fixes.last().unwrap().0,
            fixes
                .iter()
                .map(|(name, prev_ver, fix_ver)| format!("{}@{} -> {}", name, prev_ver, fix_ver))
                .collect::<Vec<String>>()
        )
        .unwrap();

        deptree.update_pkg(fixes)?;
    }
}

#[test]
fn test_audit() {
    use std::sync::{Arc, Mutex};

    const WORKSPACE_PATH: &str = "/home/ubuntu/Workspaces/Cargo-Ecosystem-Monitor/Code/cargo_ruf/ruf_audit_virtual/virt_work";
    let stdout = Arc::new(Mutex::new(std::io::stdout()));
    let mut buffer = stdout.lock().unwrap();

    // let res = audit("taxonomy", "0.3.1", WORKSPACE_PATH, &mut *buffer);
    let res = audit("pyo3", "0.9.2", WORKSPACE_PATH, &mut *buffer);
    // let res: Result<(), AuditError> = audit("byte-enum-derive", "0.1.1", WORKSPACE_PATH, &mut *buffer);

    // let res = audit("tar", "0.4.0", WORKSPACE_PATH, &mut *buffer);
    // let res = audit("chrono-tz", "0.1.0", WORKSPACE_PATH, &mut *buffer);
    // let res = audit("leaf", "0.0.1", WORKSPACE_PATH, &mut *buffer);

    println!("RESULTS: {:?}", res);
}
