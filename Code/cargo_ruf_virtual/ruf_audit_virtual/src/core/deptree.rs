use std::collections::HashMap;

use cargo_lock::dependency::Tree;

/// Record and manage the dependency tree of a crate
pub struct DepTreeManager {
    /// Inner depencency tree
    deptree: Tree,

    /// Local crates
    locals: HashMap<String, i32>
}