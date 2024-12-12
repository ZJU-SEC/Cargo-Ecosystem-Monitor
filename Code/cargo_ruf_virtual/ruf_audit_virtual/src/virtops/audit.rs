use cargo_lock::dependency::graph::NodeIndex;
use petgraph::visit;

use super::ops::DepOpsVirt;
use crate::core::{AuditError, DepTreeManager};

/// The main audit function
pub fn audit(name: &str, ver: &str, workspace: &str) -> Result<(), AuditError> {
    // Init a tree first
    let ops = DepOpsVirt::new(name, ver, workspace)?;
    let deptree = DepTreeManager::new(ops, 63)?;

    // Check if the rufs are usable and try fix if not.
    check_fix(deptree)
}

fn check_fix(mut deptree: DepTreeManager<DepOpsVirt>) -> Result<(), AuditError> {
    loop {
        // Extract current used rufs.
        let used_rufs = deptree.extract_rufs()?;

        // We do bfs and thus fix problems up to down.
        let graph = deptree.get_graph();
        let root = deptree.get_root();
        let mut issue_dep = None;

        // Check rufs topdonw.
        let mut bfs = visit::Bfs::new(&graph, root);
        while let Some(nx) = bfs.next(&graph) {
            let node = &graph[nx];
            let name_ver = format!("{}@{}", node.name, node.version);
            if let Some(rufs) = used_rufs.get(&name_ver) {
                println!(
                    "[VirtAudit Debug] check_fix: checking {}@{} rufs: {:?}",
                    node.name, node.version, rufs
                );
                if !deptree.check_rufs(rufs) {
                    // Ok here we got issues
                    issue_dep = Some((nx, node));
                    break;
                }
            }
        }

        if issue_dep.is_none() {
            // No rufs issue found (but other problem may exists).
            println!("[VirtAudit Debug] check_fix: No rufs issue found, OK!");
            return Ok(());
        }

        // Or we try to fix it.
        let (issue_depnx, issue_dep) = issue_dep.unwrap();
        println!(
            "[VirtAudit Debug] check_fix: Found issue dep: {}@{}",
            issue_dep.name, issue_dep.version
        );

        // Canditate versions, filtered by semver reqs and ruf issues.
        let candidate_vers = deptree.get_candidates(issue_depnx)?;
        println!(
            "[VirtAudit Debug] check_fix: Found {} candidates: {:?}",
            candidate_vers.len(),
            candidate_vers.iter().map(|v| v.to_string())
        );

        // Let's say, we choose the max canditate and check whether this can fix the issues.
        let choose = candidate_vers.into_iter().max();
        if let Some(fix) = choose {
            let dep_name = issue_dep.name.to_string();
            let prev_ver = issue_dep.version.to_string();
            let fix_ver = fix.to_string();

            println!(
                "[VirtAudit Debug] check_fix: Try fixing issue dep {}@{} -> {}",
                dep_name, prev_ver, fix_ver
            );
            deptree.update_pkg(&dep_name, &prev_ver, &fix_ver)?;

            // Ok, we loop back and check rufs again.
        } else {
            // Or we have to do an up fix.
            upfix(&mut deptree, issue_depnx)?;
        }
    }

    // Won't reach here, return during the loop.
}

fn upfix(
    deptree: &mut DepTreeManager<DepOpsVirt>,
    issue_depnx: NodeIndex,
) -> Result<(), AuditError> {
    // So who restrict our issue dep ?
    let strict_parent_pkgnx = match deptree.get_limit_by(issue_depnx) {
        Some(p) => p,
        None => {
            return Err(AuditError::FunctionError(
                "Up fix failed, no strict parent found".to_string(),
            ))
        }
    };

    let graph = deptree.get_graph();
    let parent_pkg = &graph[strict_parent_pkgnx];

    println!(
        "[VirtAudit Debug] upfix: No candidate found, try up fix parent: {}@{}",
        parent_pkg.name, parent_pkg.version
    );

    let parent_candidates = deptree.get_upfix_candidates(strict_parent_pkgnx, issue_depnx)?;

    println!(
        "[VirtAudit Debug] upfix: Found {} candidates: {:?}",
        parent_candidates.len(),
        parent_candidates.iter().map(|v| v.to_string())
    );

    // And we choose the max version to try fixing loose parent's req on issue_dep.
    let choose = parent_candidates.into_iter().max();
    if let Some(fix) = choose {
        let name = parent_pkg.name.to_string();
        let prev_ver = parent_pkg.version.to_string();
        let fix_ver = fix.to_string();

        println!(
            "[VirtAudit Debug] upfix: Try fixing parent {}@{} -> {}",
            parent_pkg.name, parent_pkg.version, fix
        );

        deptree.update_pkg(&name, &prev_ver, &fix_ver)?;
        // Ok, let go back.

        Ok(())
    } else {
        // Or maybe, we have to go upper and fix parent's parents.
        upfix(deptree, strict_parent_pkgnx)
    }
}

#[test]
fn test_audit() {
    const WORKSPACE_PATH: &str = "/home/ubuntu/Workspaces/Cargo-Ecosystem-Monitor/Code/cargo_ruf_virtual/ruf_audit_virtual/virt_work";

    audit("taxonomy", "0.3.1", WORKSPACE_PATH).unwrap(); // It cannot be fixed.
}
