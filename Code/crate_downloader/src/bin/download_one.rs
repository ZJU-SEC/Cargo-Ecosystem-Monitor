use std::fs::{create_dir, OpenOptions, remove_dir_all};
use std::path::Path;
use std::env;

use downloader::Downloader;
use log::warn;
use simplelog::*;

use crate_downloader::{deal_with_crate, fetch_crate};

const CRATESDIR: &str = "./download_one";


fn main() {
    // Prepare log file
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Warn,
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
                .open("./crates_downloader.log")
                .unwrap(),
        ),
    ])
    .unwrap();

    // Process input arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 3{
        println!("Input arguments should follow: <name> <version_num>. For example: rand 0.8.5");
        return;
    }
    let name = args[1].clone();
    let version_num = args[2].clone();

    // Main Process
    println!("Processing crate {name}-v{version_num}");
    remove_dir_all(CRATESDIR).unwrap_or_default(); // Delete tmp crates file directory
    create_dir(Path::new(CRATESDIR)).unwrap_or_default(); // Crates file directory
    let mut downloader = Downloader::builder()
        .download_folder(Path::new(CRATESDIR))
        .parallel_requests(1)
        .build()
        .expect("Fatal Error, build downloader fails!");

    if let Err(e) =
        fetch_crate( &mut downloader, CRATESDIR, &name, &version_num)
    {
        warn!("Fetch fails: {}",  e);
    } else if let Err(e) = deal_with_crate(CRATESDIR, &name, &version_num) {
        warn!("Unzip fails: {}", e);
    } else {
        println!("Success.");
    }

}
