extern crate crossbeam;
extern crate simplelog;

use simplelog::*;
use std::fs::OpenOptions;

mod util;
use util::run_audit;

fn main() {
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
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
                .open("./ruf_audit.log")
                .unwrap(),
        ),
    ])
    .unwrap();

    run_audit(3, "undone");
}
