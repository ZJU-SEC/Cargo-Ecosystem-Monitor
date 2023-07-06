use std::env;

use rust_deps::{resolve_deps_of_version_once};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 3{
        let name = args[1].clone();
        let num = args[2].clone();
        let deps = resolve_deps_of_version_once(name, num).unwrap();
        println!("{deps}");
    }
    else {
        println!("Input arguments should follow: <name> <version_num>. For example: rand 0.8.5");
    }
}