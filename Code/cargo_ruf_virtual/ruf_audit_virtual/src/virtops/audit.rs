use fxhash::FxHashMap;
use petgraph::visit;

use super::ops::DepOpsVirt;
use crate::core::{AuditError, DepTreeManager};

/// The main audit function
pub fn audit(name: &str, ver: &str, version_id: i32) -> Result<(), AuditError> {
    // Init a tree first
    let ops = DepOpsVirt::new(name, ver, version_id)?;
    let deptree = DepTreeManager::new(ops, 63);

    let used_rufs = deptree.extract_rufs()?;

    // Check if the rufs are usable and try fix if not.
    check_fix(deptree, used_rufs)
}

fn check_fix(
    deptree: DepTreeManager<DepOpsVirt>,
    used_rufs: FxHashMap<String, Vec<String>>,
) -> Result<(), AuditError> {
    loop {
        // We do bfs and thus fix problems up to down.
        let graph = deptree.get_graph();
        let root = deptree.get_root();
        let mut issue_dep = None;

        // Check rufs topdonw
        let mut bfs = visit::Bfs::new(&graph, root);
        while let Some(nx) = bfs.next(&graph) {
            let node = &graph[nx];
            if let Some(rufs) = used_rufs.get(node.name.as_str()) {
                if !deptree.check_rufs(rufs) {
                    // Ok here we got issues
                    issue_dep = Some((nx, node));
                    break;
                }
            }
        }

        if issue_dep.is_none() {
            // No rufs issue found (but other problem may exists).
            return Ok(());
        }

        // Or we try to fix it.
        let (issue_depnx, issue_dep) = issue_dep.unwrap();

        // Canditate versions, restricted by semver reqs.
        let candidate_vers = deptree.get_candidates(issue_depnx)?;

        // FIXME: impl audit
    }

    unimplemented!()
}
