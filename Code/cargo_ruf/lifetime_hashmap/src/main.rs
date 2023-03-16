use std::collections::HashMap;
use lifetime_hashmap::{get_lifetime, get_ruf_status};

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
