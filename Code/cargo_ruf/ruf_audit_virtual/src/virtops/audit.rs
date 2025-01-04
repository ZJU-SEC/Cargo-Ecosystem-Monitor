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

    // Check issues and fix them.
    check_fix(deptree, debugger)
}

fn check_fix(
    mut deptree: DepTreeManager<DepOpsVirt>,
    debugger: &mut impl Write,
) -> Result<(), AuditError> {
    for rustc in 64..0 {
        deptree.set_rustv(rustc);
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
            try_fix(&mut deptree, fixes)?;
        } else {
            writeln!(
                debugger,
                "[VirtAudit Debug] check_fix: Rustc version {} got issues cannot be fixed.",
                rustc
            )
            .unwrap();
        }
    }
    unimplemented!()
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
    deptree.set_local(&root);

    // Collect all ruf issues first.
    let mut issue_deps = Vec::new();
    let mut bfs = visit::Bfs::new(&graph, root);
    while let Some(nx) = bfs.next(&graph) {
        let node = &graph[nx];
        let name_ver = format!("{}@{}", node.name, node.version);
        if let Some(rufs) = used_rufs.get(&name_ver) {
            writeln!(
                debugger,
                "[VirtAudit Debug] check_fix: Checking ruf enabled package {}@{} rufs: {:?}",
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
) -> Result<(), AuditError> {
    unimplemented!()
}

#[test]
fn test_audit() {
    use std::sync::{Arc, Mutex};

    const WORKSPACE_PATH: &str = "/home/ubuntu/Workspaces/Cargo-Ecosystem-Monitor/Code/cargo_ruf/ruf_audit_virtual/virt_work";
    let stdout = Arc::new(Mutex::new(std::io::stdout()));
    let mut buffer = stdout.lock().unwrap();

    // let res = audit("taxonomy", "0.3.1", WORKSPACE_PATH, &mut *buffer);
    // let res = audit("pyo3", "0.9.2", WORKSPACE_PATH, &mut *buffer);
    // let res: Result<(), AuditError> = audit("byte-enum-derive", "0.1.1", WORKSPACE_PATH, &mut *buffer);

    let res = audit("tar", "0.4.0", WORKSPACE_PATH, &mut *buffer);
    // let res = audit("chrono-tz", "0.1.0", WORKSPACE_PATH, &mut *buffer);
    // let res = audit("leaf", "0.0.1", WORKSPACE_PATH, &mut *buffer);

    println!("RESULTS: {:?}", res);
}
