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
                .open("./rust_deps.log")
                .unwrap(),
        ),
    ])
    .unwrap();

    run_deps(6)

    // let start = Instant::now();

    // use cargo::core::registry::PackageRegistry;
    // use cargo::core::resolver::{CliFeatures, HasDevUnits};
    // use cargo::core::Workspace;
    // use cargo::ops;
    // use cargo::util::Config;
    // use cargo::core::Shell;

    // use std::path::Path;
    // use std::thread;

    // let mut handles = Vec::new();
    // for i in 0..6 {
    //     handles.push(thread::spawn(move || {
    //         // let config = Config::default().unwrap();
    //         let config = Config::new(
    //             Shell::new(),
    //             env::current_dir().unwrap(),
    //             format!("/Users/wyffeiwhe/Desktop/Research/Code/rust_deps/job{}", i).into(),
    //         );

    //         // config
    //         //     .configure(0, false, None, false, false, true, &None, &[], &[])
    //         //     .unwrap();

    //         let ws = Workspace::new(
    //             &Path::new(&format!(
    //                 "/Users/wyffeiwhe/Desktop/Research/Code/rust_deps/dep{}.toml",
    //                 i
    //             )),
    //             &config,
    //         )
    //         .unwrap();

    //         let mut registry = PackageRegistry::new(ws.config()).unwrap();
    //         let resolve = ops::resolve_with_previous(
    //             &mut registry,
    //             &ws,
    //             &CliFeatures::new_all(true),
    //             HasDevUnits::Yes,
    //             None,
    //             None,
    //             &[],
    //             true,
    //         )
    //         .unwrap();

    //         println!("Done job {}", i)
    //     }));
    // }

    // for handle in handles {
    //     handle.join().unwrap();
    // }

    // println!("Total Time: {}", start.elapsed().as_secs());
}
