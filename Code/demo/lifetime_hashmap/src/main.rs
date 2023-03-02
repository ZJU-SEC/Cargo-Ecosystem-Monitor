use std::collections::HashMap;
use lifetime::{RUSTC_VER_NUM, get_lifetime_raw};
mod lifetime;

fn main() {
    let lifetime = get_lifetime();
    // let test_status = get_ruf_status(&lifetime, "string_leak", 0);
    // println!("{:#?}", lifetime);
    println!("RUF(string_leak) in version 1.0.0 :{:#?}", get_ruf_status(&lifetime, "string_leak", 0));
    println!("RUF(string_leak) in version 1.57.0 :{:#?}", get_ruf_status(&lifetime, "string_leak", 57));
    println!("RUF(string_leak) in version 1.67.0 :{:#?}", get_ruf_status(&lifetime, "string_leak", 67));
    println!("RUF(string_leak) in version 1.77.0 :{:#?}", get_ruf_status(&lifetime, "string_leak", 77));
    println!("RUF(vec_deque_retain) in version 1.0.0 :{:#?}", get_ruf_status(&lifetime, "vec_deque_retain", 0));
    println!("RUF(vec_deque_retain) in version 1.1.0 :{:#?}", get_ruf_status(&lifetime, "vec_deque_retain", 1));
    println!("RUF(vec_deque_retain) in version 1.5.0 :{:#?}", get_ruf_status(&lifetime, "vec_deque_retain", 5));
}


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

/// Get RUF Lifetime table:
///     Data structure: HashMap< RUF Name, Status[rustc_version]>
///     Access: status = HashMap[RUF].[rustc_version]
///     Return: The RUF status of given RUF in the specific rustc_version (0 represents rustc-1.0.0, 67 for rustc-1.67.0). 
pub fn get_lifetime () -> HashMap<&'static str, [&'static str; RUSTC_VER_NUM]> {
    get_lifetime_raw()
}

