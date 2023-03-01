extern crate anyhow;
extern crate cargo;


mod util;
use util::resolve;


const RUSTC: &str = "/Users/wyffeiwhe/Desktop/Research/Supplychain/Cargo-Ecosystem-Monitor/rust/build/x86_64-apple-darwin/stage1/bin/rustc";

fn main() {
    let r = resolve("/Users/wyffeiwhe/Desktop/Research/Supplychain/Cargo-Ecosystem-Monitor/Code/cargo_tools/demo", "demo");
    println!("{:?}", r);
}
