extern crate simplelog;
extern crate anyhow;

use simplelog::*;
use std::fs::File;
use util::*;

mod util;

fn main() {
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Info,
            Config::default(),
            File::create("./rust_deps.log").unwrap(),
        ),
    ])
    .unwrap();
    run_deps(20)
}
