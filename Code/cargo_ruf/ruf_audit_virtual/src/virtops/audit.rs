use std::io::Write;

use cargo_lock::dependency::graph::NodeIndex;
use petgraph::visit;

use super::ops::DepOpsVirt;
use crate::{
    basic::RUSTC_VER_NUM,
    core::{AuditError, DepTreeManager},
};

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
    loop {
        // Extract current used rufs.
        let used_rufs = deptree.extract_rufs()?;

        // We do bfs and thus fix problems up to down.
        let graph = deptree.get_graph();
        // In virt audit, real root is the child of `root`.
        let root = graph.neighbors(deptree.get_root()).next().unwrap();

        writeln!(debugger, "[VirtAudit Debug] check_fix: root {:?}", root).unwrap();
        deptree.set_local(root);

        let mut issue_dep = None;

        // Check rufs top-donw.
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
                    issue_dep = Some((nx, node.name.to_string(), node.version.to_string()));
                    break;
                }
            }
        }

        if issue_dep.is_none() {
            // No rufs issue found (but other problem may exists).
            writeln!(
                debugger,
                "[VirtAudit Debug] check_fix: No rufs issue found, OK!"
            )
            .unwrap();
            return Ok(());
        }

        let issue_dep = issue_dep.unwrap();
        let issue_name_ver = format!("{}@{}", issue_dep.1, issue_dep.2);

        writeln!(
            debugger,
            "[VirtAudit Debug] check_fix: Found issue dep: {}",
            issue_name_ver
        )
        .unwrap();

        let fixable = deptree.issue_fixable(issue_dep.0, debugger);
        writeln!(
            debugger,
            "[VirtAudit Debug] check_fix: Issue fixable: {:?}",
            fixable
        )
        .unwrap();
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
