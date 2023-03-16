pub mod lifetime;
pub const RUSTC_VER_NUM:usize = 68;

use lifetime::{get_lifetime_raw};
use std::collections::HashMap;


/// Get RUF status given the lifetime table:
///     Input: lifetime:Rust Lifetime Table, ruf: query ruf name, rustc_version (0 represents rustc-1.0.0, 67 for rustc-1.67.0)
///     Return: Some(&str) for existent RUF status, None for nonexistent rustc_version/ruf/status.
pub fn get_ruf_status(lifetime: &HashMap<&'static str, [&'static str; RUSTC_VER_NUM]>, ruf: &str, rustc_version:usize) -> Option<&'static str> {
    // No such rustc_version
    if rustc_version >= RUSTC_VER_NUM {
        return None;
    }
    if let Some(status_vec) = lifetime.get(ruf){
        let status = status_vec[rustc_version];
        // RUF not defined in the version
        if status == "None" {
            return None;
        }
        // RUF defined
        else{
            return Some(status);
        }
    }
    // No RUF found
    None
}

/// Get RUF status (all versions)
pub fn get_ruf_status_all(lifetime:&HashMap<&'static str, [&'static str; RUSTC_VER_NUM]>, ruf: &str) -> Option<Vec<&'static str>> {
    if let Some(status_vec) = lifetime.get(ruf){
        return Some(status_vec.to_vec());
    } else {
        return None;
    }
}

/// Get RUF Lifetime table:
///     Data structure: HashMap< RUF Name, Status[rustc_version]>
///     Access: status = HashMap[RUF].[rustc_version]
///     Return: The RUF status of given RUF in the specific rustc_version (0 represents rustc-1.0.0, 67 for rustc-1.67.0). 
pub fn get_lifetime () -> HashMap<&'static str, [&'static str; RUSTC_VER_NUM]> {
    get_lifetime_raw()
}

