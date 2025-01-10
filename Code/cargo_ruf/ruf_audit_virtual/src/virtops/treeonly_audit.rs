use std::io::Write;

use cargo_lock::dependency::graph::NodeIndex;
use fxhash::{FxHashMap, FxHashSet};
use petgraph::visit;
use semver::Version;

use super::ops::DepOpsVirt;
use crate::core::{AuditError, DepTreeManager};

#[derive(Debug)]
pub struct Summary {
    pub fix_rustv: i32,
    pub fix_deps: FxHashMap<String, Vec<(String, Version, Version)>>,
}

/// The main audit function.
/// The debugger receives an output stream to write debug information.
pub fn treeonly_audit(
    name: &str,
    ver: &str,
    workspace: &str,
    debugger: &mut impl Write,
) -> Result<Summary, AuditError> {
    // Init a tree first
    let ops = DepOpsVirt::new(name, ver, workspace)?;
    let mut deptree = DepTreeManager::new(ops, 63)?;
    let logical_root = deptree
        .get_graph()
        .neighbors(deptree.get_root())
        .next()
        .unwrap();
    deptree.set_local(&logical_root);

    check_fix(deptree, debugger)
}

fn check_fix(
    mut deptree: DepTreeManager<DepOpsVirt>,
    debugger: &mut impl Write,
) -> Result<Summary, AuditError> {
    let rustc = 63;
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
        return Ok(Summary {
            fix_rustv: rustc,
            fix_deps: FxHashMap::default(),
        });
    }

    let first_issue = issue_deps.first().cloned().unwrap();
    match check_fixable(&mut deptree, issue_deps, debugger) {
        Ok(mut fixes) => match try_fix(&mut deptree, first_issue, fixes.remove(0), debugger) {
            Ok(fix_deps) => {
                writeln!(
                    debugger,
                    "[VirtAudit Debug] check_fix: Rustc version {} issues fixed.",
                    rustc,
                )
                .unwrap();
                return Ok(Summary {
                    fix_rustv: rustc,
                    fix_deps,
                });
            }
            Err(e) => {
                writeln!(debugger,
                        "[VirtAudit Debug] check_fix: Fix failure for rustc version {} with issue: {:?}", rustc, e).unwrap();
                return Err(e);
            }
        },
        Err(e) => {
            writeln!(debugger,
                "[VirtAudit Debug] check_fix: Rustc version {} got issues cannot be fixed with error: {:?}.", rustc, e).unwrap();
            return Err(e);
        }
    }
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
) -> Result<Vec<Vec<(String, Version, Version)>>, AuditError> {
    let graph = deptree.get_graph();
    let mut fixes = Vec::new();

    // Check possible fix for each issue.
    for nx in issue_deps {
        writeln!(
            debugger,
            "[VirtAudit Debug] check_fixable: Check {}@{} fixibility",
            graph[nx].name, graph[nx].version,
        )
        .unwrap();
        match deptree.issue_fixable(nx, debugger) {
            Ok(fix) => {
                let fix = fix
                    .into_iter()
                    .map(|(nx, fix_ver)| {
                        (
                            graph[nx].name.to_string(),
                            graph[nx].version.clone(),
                            fix_ver,
                        )
                    })
                    .collect::<Vec<_>>();

                writeln!(
                    debugger,
                    "[VirtAudit Debug] check_fixable: Issue dep {}@{} is fixable with {:?}, adds to fix limits.",
                    graph[nx].name,
                    graph[nx].version,
                    fix.iter()
                        .map(|(name, ver, fix_ver)| format!("{}@{} -> {}", name, ver, fix_ver))
                        .collect::<Vec<_>>()
                )
                .unwrap();

                // Add limits for the fix.
                deptree.set_fix_limit(&fix);

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

    // Ok clear the limit.
    deptree.clear_fix_limit();

    Ok(fixes)
}

fn try_fix(
    deptree: &mut DepTreeManager<DepOpsVirt>,
    first_issue: NodeIndex,
    first_fix: Vec<(String, Version, Version)>,
    debugger: &mut impl Write,
) -> Result<FxHashMap<String, Vec<(String, Version, Version)>>, AuditError> {
    // For loop detect.
    let mut already_fixed = FxHashSet::default();
    let mut is_first = Some((first_issue, first_fix));
    let mut fix_deps = FxHashMap::default();

    // The fix modify the deptree, and thus the remaining issues and their fixability may changes.
    // So here we have to recheck the issues and fix them.
    loop {
        let graph = deptree.get_graph();

        let (issue_nx, fix) = if is_first.is_some() {
            is_first.take().unwrap()
        } else {
            let issue_nx = check_issue(deptree, debugger)?.first().cloned();
            if issue_nx.is_none() {
                return Ok(fix_deps);
            }

            let issue_nx = issue_nx.unwrap();

            let fix = deptree.issue_fixable(issue_nx, debugger)?;
            assert!(!fix.is_empty(), "Fatal, no fix found when fixing issue.");

            let fix = fix
                .into_iter()
                .map(|(nx, fix_ver)| {
                    (
                        graph[nx].name.to_string(),
                        graph[nx].version.clone(),
                        fix_ver,
                    )
                })
                .collect::<Vec<_>>();
            (issue_nx, fix)
        };

        let issue_name_ver = format!("{}@{}", graph[issue_nx].name, graph[issue_nx].version);
        writeln!(
            debugger,
            "[VirtAudit Debug] try_fix: Try fix {} with {:?}",
            graph[issue_nx].name,
            fix.iter()
                .map(|(name, ver, fix_ver)| format!("{}@{} -> {}", name, ver, fix_ver))
                .collect::<Vec<_>>()
        )
        .unwrap();

        let entry = fix_deps
            .entry(issue_name_ver.clone())
            .or_insert_with(Vec::new);

        // Set the limit first.
        deptree.set_fix_limit(&fix);
        let steps = deptree.issue_dofix(issue_nx, fix, debugger)?;

        entry.extend(steps.into_iter());

        let check_loop = already_fixed.insert(issue_name_ver);

        if !check_loop {
            return Err(AuditError::FunctionError(
                Some(format!("Dupfix, maybe a loop happens",)),
                Some(issue_nx),
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
    let res = treeonly_audit("leaf", "0.0.1", WORKSPACE_PATH, &mut *buffer);

    // let res = audit("kunai", "0.3.0", WORKSPACE_PATH, &mut *buffer);
    // let res = audit("hsr-codegen", "0.2.0", WORKSPACE_PATH, &mut *buffer);
    // let res = audit("bouncer", "1.0.0", WORKSPACE_PATH, &mut *buffer);
    // let res = audit("tari_comms_dht", "0.8.1", WORKSPACE_PATH, &mut *buffer);

    println!("RESULTS: {:?}", res);
}
