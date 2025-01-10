use simplelog::*;
use std::fs::OpenOptions;

mod utils;

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
                .open("./virt_audit.log")
                .unwrap(),
        ),
    ])
    .unwrap();

    utils::run_audit_virt(3, "undone");
}