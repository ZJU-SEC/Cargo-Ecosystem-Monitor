use std::io::Write;

use super::ops::DepOpsVirt;
use crate::core::{AuditError, DepTreeManager};

/// This is only for audit evaluations. We check whethe a crate can be fixed by rustc, and only take consider of its root.
pub fn root_audit(
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

    let used_rufs = deptree.extract_rufs()?;
    if let Some(root_used_rufs) = used_rufs.get(&format!("{name}@{ver}")) {
        for rustv in (0..64).rev() {
            deptree.switch_rustv(rustv);
            let issue_rufs = deptree.filter_rufs(root_used_rufs.iter().collect());
            if issue_rufs.is_empty() {
                writeln!(
                    debugger,
                    "[Root Audit] Rustc {} fixed root crate {}@{}",
                    rustv, name, ver
                )
                .unwrap();

                return Ok(rustv);
            } else {
                writeln!(
                    debugger,
                    "[Root Audit] Rustc {} cannot fix root crate {}@{} due to {:?}",
                    rustv, name, ver, issue_rufs
                )
                .unwrap();
            }
        }

        return Err(AuditError::FunctionError(None, None));
    } else {
        writeln!(
            debugger,
            "[Root Audit] No rufs found for root crate: {}@{}",
            name, ver
        )
        .unwrap();

        return Ok(63);
    }
}

#[test]
fn test_audit() {
    use std::sync::{Arc, Mutex};

    const WORKSPACE_PATH: &str = "/home/ubuntu/Workspaces/Cargo-Ecosystem-Monitor/Code/cargo_ruf/ruf_audit_virtual/virt_work";
    let stdout = Arc::new(Mutex::new(std::io::stdout()));
    let mut buffer = stdout.lock().unwrap();

    let res = root_audit("capnp", "0.0.2", WORKSPACE_PATH, &mut *buffer);

    println!("RESULTS: {:?}", res);
}
