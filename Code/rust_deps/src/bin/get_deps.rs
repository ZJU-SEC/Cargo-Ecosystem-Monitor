use std::env;

use rust_deps::{resolve_deps_of_version_once, resolve_deps_of_version_once_full, test_registry};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 3{
        let name = args[1].clone();
        let num = args[2].clone();
        let deps = resolve_deps_of_version_once(name, num).unwrap();
        println!("{deps}");
        return;
    }
    if args.len() == 4 && args[1] == "full"{
        let name = args[2].clone();
        let num = args[3].clone();
        let deps = resolve_deps_of_version_once_full(name, num).unwrap();
        println!("{deps}");
        return;
    }
    if args.len() == 4 && args[1] == "test"{
        let name = args[2].clone();
        let num = args[3].clone();
        let deps = test_registry(name, num).unwrap();
        println!("{deps}");
        return;
    }
    println!("Input arguments should follow: <name> <version_num>. For example: rand 0.8.5");
    println!("Or: full <name> <version_num>, if you want to get full dep info.");
}