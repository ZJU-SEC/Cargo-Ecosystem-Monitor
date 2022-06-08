mod util;

use simplelog::*;
use std::fs::OpenOptions;
use util::run;

fn main() {
    let mut config = ConfigBuilder::new();
    let config = match config.set_time_offset_to_local() {
        Ok(local) => local.build(),
        Err(_) => config.build(),
    };

    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            config.clone(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Warn,
            config,
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .append(true)
                .open("./run_propagation.log")
                .unwrap(),
        ),
    ])
    .unwrap();

    run(12);
}
