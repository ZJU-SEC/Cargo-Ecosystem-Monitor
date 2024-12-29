use std::{cell::RefCell, io::Write, rc::Rc};

use cargo::core::Resolve;
use cargo_lock::dependency::{
    graph::{EdgeDirection, Graph, NodeIndex},
    Tree,
};
use fxhash::FxHashMap;
use petgraph::visit::EdgeRef;
use semver::Version;

use crate::{
    basic::{get_all_ruf_status, RufStatus, RUSTC_VER_NUM},
    core::{depops::DepOps, error::AuditError},
};

pub type UsedRufs = FxHashMap<String, Vec<String>>;

/// Record and manage the dependency tree of a crate
pub struct DepTreeManager<D: DepOps> {
    /// Rustc versions.
    rustv: u32,

    /// Depencency operators
    depops: D,
    ///Dependency resolve related info
    depresolve: Rc<(Resolve, Tree, UsedRufs)>,

    /// Dependency limited-by lists, for upfix
    dep_limit_by: RefCell<FxHashMap<NodeIndex, NodeIndex>>,
    /// Audit mid-result records
    issue_stacks: RefCell<Vec<(Rc<(Resolve, Tree, UsedRufs)>, [bool; RUSTC_VER_NUM])>>,

    /// Triable rustc versions
    overall_rustc_matrix: RefCell<[bool; RUSTC_VER_NUM]>,
}

impl<D: DepOps> DepTreeManager<D> {
    /// Create new DepTreeManager from current configurations.
    pub fn new(ops: D, rustv: u32) -> Result<Self, AuditError> {
        let (resolve, tree) = ops.first_resolve()?;
        let used_rufs = ops.extract_rufs(&resolve)?;

        Ok(Self {
            rustv: rustv,

            depops: ops,
            depresolve: Rc::new((resolve, tree, used_rufs)),

            dep_limit_by: RefCell::new(FxHashMap::default()),
            issue_stacks: RefCell::new(Vec::new()),

            overall_rustc_matrix: RefCell::new([true; RUSTC_VER_NUM]),
        })
    }

    pub fn extract_rufs(&self) -> Result<UsedRufs, AuditError> {
        Ok(self.depresolve.2.clone())
    }

    pub fn check_rufs(&self, rufs: &Vec<String>) -> bool {
        self.depops.check_rufs(self.rustv, rufs)
    }

    pub fn get_graph(&self) -> &Graph {
        self.depresolve.1.graph()
    }

    pub fn get_root(&self) -> NodeIndex {
        let roots = self.depresolve.1.roots();
        assert!(roots.len() == 1, "Fatal, multiple roots found");
        roots[0]
    }

    pub fn get_rustv(&self) -> u32 {
        self.rustv
    }

    pub fn get_dep_limit_by(&self, pkgnx: NodeIndex) -> Option<NodeIndex> {
        self.dep_limit_by.borrow().get(&pkgnx).cloned()
    }

    /// Get usable candidates of a node that match it's parents' version req, and free from rufs issues.
    pub fn get_candidates(
        &self,
        pkgnx: NodeIndex,
        debugger: &mut impl Write,
    ) -> Result<Vec<Version>, AuditError> {
        let graph = self.get_graph();
        let dep = &graph[pkgnx];
        let dep_name = dep.name.to_string();
        let dep_ver = dep.version.to_string();

        let parents = self.get_parents(pkgnx);
        assert!(parents.len() >= 1, "Fatal, root has no parents");

        let candidates = self.depops.get_all_candidates(&dep_name)?;
        assert!(!candidates.is_empty());

        // Collect parents' version req on current package.
        let mut version_reqs = Vec::new();
        for p in parents {
            let p_pkg = &graph[p];
            let p_name = p_pkg.name.as_str();
            let p_ver = p_pkg.version.to_string();

            let mut meta = self.depops.get_pkg_versionreq(p_name, &p_ver)?;
            let req = meta
                .remove(&dep_name)
                .expect("Fatal, cannot find dependency in parent package");
            // prepare for relaxing strict parents.

            writeln!(
                debugger,
                "[Deptree Debug] get_candidates: check {}@{} with parent {}@{} req: {}",
                dep_name, dep_ver, p_name, p_ver, req
            )
            .unwrap();

            let lowest = candidates
                .keys()
                .filter(|key| req.matches(key))
                .min()
                .cloned()
                .expect("Fatal, cannot find lowest allowing version");
            version_reqs.push((p, req, lowest));
        }

        // We assume parents who restricts the version most is the one with max min_lowest,
        // and it shall be updated later, if we need up fix.
        // This assumption won't hold for all cases (cases with complex version req),
        // but most of the times it works.
        let strict_parent = version_reqs
            .iter()
            .max_by_key(|&(_, _, v)| v)
            .expect("Fatal, no strict parent found");

        self.dep_limit_by
            .borrow_mut()
            .insert(pkgnx, strict_parent.0);

        // we choose candidates as:
        // 1. match its dependents' version req
        // 2. smaller than current version
        // 3. free from ruf issues
        // we will record who restricts the version most, for later up fix.
        let mut usable = Vec::new();
        for (ver, condrufs) in candidates.into_iter().filter(|(ver, _)| {
            version_reqs.iter().all(|(_, req, _)| req.matches(ver)) && ver < &dep.version
        }) {
            let rufs =
                self.depops
                    .resolve_condrufs(&self.depresolve.0, &dep_name, &dep_ver, condrufs)?;

            let issue_rufs = self.depops.filter_issue_rufs(self.rustv, rufs.clone());

            writeln!(
                debugger,
                "[Deptree Debug] get_candidates: check req-matched version {} with rufs: {:?}, issue: {:?}",
                ver, rufs, issue_rufs
            )
            .unwrap();

            if issue_rufs.is_empty() {
                usable.push(ver);
            }
        }

        Ok(usable)
    }

    /// Used in up fix, similar to [`get_candidates`], but get parents' candidates with older version req
    /// to the dep package, so we can do relax.
    pub fn get_upfix_candidates(
        &self,
        parent_pkgnx: NodeIndex,
        dep_pkgnx: NodeIndex,
        debugger: &mut impl Write,
    ) -> Result<Vec<Version>, AuditError> {
        let graph = self.get_graph();

        let parent_pkg = &graph[parent_pkgnx];
        let parent_name = parent_pkg.name.as_str();
        let parent_ver = parent_pkg.version.to_string();

        let dep_pkg = &graph[dep_pkgnx];
        let dep_name = dep_pkg.name.as_str();

        let parent_candidates = self.get_candidates(parent_pkgnx, debugger)?;

        // Find out parent version with older version req to dep package.
        let cur_req = self
            .depops
            .get_pkg_versionreq(parent_name, &parent_ver)?
            .remove(dep_name)
            .expect("Fatal, cannot find dependency in parent package");

        let mut usable = vec![];
        for cad in parent_candidates {
            let mut reqs = self
                .depops
                .get_pkg_versionreq(parent_name, cad.to_string().as_str())?;

            writeln!(
                debugger,
                "[Deptree Debug] get_upfix_candidates: check version {} cur_req: {}, new_req: {:?}",
                cad,
                cur_req,
                reqs.get(dep_name).map(|req| req.to_string())
            )
            .unwrap();

            if let Some(req) = reqs.remove(dep_name) {
                // We take the assumption that, older verison shall have looser semver req,
                // so if req differs, we assume it's a candidate, since semver comparision can be hard.
                if req != cur_req {
                    usable.push(cad);
                }
            } else {
                // dep not found, possibily not used, thus ok too.
                usable.push(cad);
            }
        }

        Ok(usable)
    }

    /// Found an issue, record it in the stacks for possible later usage.
    pub fn found_issue(&self, issue_name_ver: &str) {
        let used_ruf = self.depresolve.2.get(issue_name_ver).unwrap();
        let rustc_matrix = self.resolve_rustc_matrix(used_ruf);

        self.issue_stacks
            .borrow_mut()
            .push((self.depresolve.clone(), rustc_matrix));
    }

    /// When deptree fix failed, rustc switch is needed. To decide which rustc
    /// to switch to, and which resolve we use to continue,
    /// we need to check the issue stacks, and find the best rustc and resolve.
    pub fn get_issues(
        &self,
    ) -> std::cell::Ref<Vec<(Rc<(Resolve, Tree, UsedRufs)>, [bool; RUSTC_VER_NUM])>> {
        self.issue_stacks.borrow()
    }

    /// Update a package in the dependency tree.
    pub fn update_pkg(
        &mut self,
        name: &str,
        prev_ver: &str,
        new_ver: &str,
    ) -> Result<(), AuditError> {
        let (new_resolve, new_tree) =
            self.depops
                .update_resolve(&self.depresolve.0, name, prev_ver, new_ver)?;
        let used_rufs = self.depops.extract_rufs(&new_resolve)?;

        self.depresolve = Rc::new((new_resolve, new_tree, used_rufs));
        self.dep_limit_by.borrow_mut().clear();

        Ok(())
    }

    /// Since we are rustc-oriented, we always wants newer rustc, and thus during the fix,
    /// we will try deptree fix first with newer rustc, and do rustc switch when failure.
    ///
    /// During the switch, the continue point matters.
    pub fn update_rustc(&mut self, rustv: u32, resolve_id: usize) {
        self.rustv = rustv;
        self.depresolve = self.issue_stacks.borrow()[resolve_id].0.clone();

        // Updates the overall rustc matrix, record those impossible.
        let mut overall_rustc_matrix = self.overall_rustc_matrix.borrow_mut();
        for (_, matrix) in self.issue_stacks.borrow().iter() {
            for i in 0..RUSTC_VER_NUM {
                overall_rustc_matrix[i] |= matrix[i];
            }
        }

        self.issue_stacks.borrow_mut().clear();
        self.dep_limit_by.borrow_mut().clear();
    }

    /// Get the parents of a node in the dependency tree.
    fn get_parents(&self, depnx: NodeIndex) -> Vec<NodeIndex> {
        self.depresolve
            .1
            .graph()
            .edges_directed(depnx, EdgeDirection::Incoming)
            .map(|edge| edge.source())
            .collect()
    }

    /// Resolve the triable rustc matrix for the given used rufs.
    fn resolve_rustc_matrix(&self, used_rufs: &Vec<String>) -> [bool; RUSTC_VER_NUM] {
        let mut matrix = [true; RUSTC_VER_NUM];
        for ruf in used_rufs {
            let status = get_all_ruf_status(ruf);
            let new_matrix = explain_status(status);
            merge_matrix(&mut matrix, &new_matrix);
        }
        merge_matrix(&mut matrix, &self.overall_rustc_matrix.borrow());

        return matrix;

        fn explain_status(status: [RufStatus; RUSTC_VER_NUM]) -> [bool; RUSTC_VER_NUM] {
            let mut matrix = [true; RUSTC_VER_NUM];
            for (index, s) in status.iter().enumerate() {
                if s.is_usable() {
                    matrix[index] = true;
                } else {
                    matrix[index] = false;
                }
            }

            return matrix;
        }

        fn merge_matrix(matrix: &mut [bool; RUSTC_VER_NUM], new_matrix: &[bool; RUSTC_VER_NUM]) {
            for i in 0..RUSTC_VER_NUM {
                matrix[i] &= new_matrix[i];
            }
        }
    }
}
