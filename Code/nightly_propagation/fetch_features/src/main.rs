extern crate downloader;
extern crate anyhow;
extern crate crossbeam;
extern crate simplelog;
extern crate tar;
extern crate flate2;
extern crate toml;

mod util;

use simplelog::*;
use std::fs::OpenOptions;
use util::{run, run_offline};

// const CRATEDIR: &str = "path/to/sourse_code";
const CRATEDIR: &str = "/app/Code/crate_downloader/on_process";

fn main() {
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Error,
            simplelog::Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Warn,
            simplelog::Config::default(),
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .append(true)
                .open("./fetch_crates.log")
                .unwrap(),
        ),
    ])
    .unwrap();

    // run(5, "undone")
    run_offline(20, "undone", CRATEDIR)
}
