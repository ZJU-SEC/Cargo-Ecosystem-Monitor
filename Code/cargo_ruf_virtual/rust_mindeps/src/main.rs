extern crate anyhow;
extern crate crossbeam;
extern crate simplelog;

use simplelog::*;
use std::fs::OpenOptions;
use util::*;

mod util;

fn main() {
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Warn,
            simplelog::Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Info,
            simplelog::Config::default(),
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .append(true)
                .open("./rust_mindeps.log")
                .unwrap(),
        ),
    ])
    .unwrap();

    run_deps(10, "undone");
    // run_deps(20, "processing");
}
