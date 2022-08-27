
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::fs::{create_dir, remove_dir_all, File, OpenOptions};
use std::path::Path;
use std::io::prelude::*;

use postgres::{Client, NoTls};

use accuracy_evaluation::tools::helper::{CRATESDIR, OUTPUTDIR};

pub const ALLRESULTSDIR:&str = "./results";

/// The autorun process will automatically do 
/// benchmark_dataset, pipeline_evaluation, results_summary in different dataset.
/// Procedure:
///     1. Delete `output` directory
///     2. Run evaluation process one by one.
///     3. Drop process status DB table
///     4. Store results locally 
///     5. Repeat in each dataset
fn main()-> std::io::Result<()> {
    // 1. Delete `output` directory
    {
        remove_dir_all(OUTPUTDIR).unwrap_or_default();
    }
                        
    let conn = Arc::new(Mutex::new(
        Client::connect(
            "host=localhost dbname=crates user=postgres password=postgres",
            NoTls,
        )
        .unwrap(),
    ));
    
    // 2. Run evaluation process one by one.
    remove_dir_all(ALLRESULTSDIR).unwrap_or_default();// Delete tmp crates file directory
    create_dir(Path::new(ALLRESULTSDIR)).unwrap_or_default(); // Crates file directory
    let datasets = ["hot", "random", "mostdir"];
    for dataset in datasets {
        conn.lock().unwrap().
            query("DROP TABLE IF EXISTS accuracy_evaluation_status", &[]).unwrap();
        println!("Start dataset: {}", dataset);
        Command::new("cargo").arg("run")
                .arg("--bin")
                .arg("benchmark_dataset")
                .arg("--")
                .arg(dataset)
                .stdout(Stdio::inherit())
                .output()
                .expect("failed to run");
        println!("Finish dataset generation: {}", dataset);
        Command::new("cargo").arg("run")
                .arg("--bin")
                .arg("pipeline_evaluation")
                .output().expect("pipeline_evaluation exec error!");
        let results = Command::new("cargo").arg("run")
                            .arg("--bin")
                            .arg("results_summary")
                            .output().expect("results_summary exec error!");
        // 4. Store results locally
        create_dir(Path::new(&format!("{}/{}", ALLRESULTSDIR, dataset))).unwrap_or_default(); // Crates file directory
        Command::new("mv")
                .arg(OUTPUTDIR)
                .arg(&format!("{}/{}/", ALLRESULTSDIR, dataset))
                .output().expect("mv source code error!");
        let mut result_file = File::create(Path::new(&format!("{}/{}/results.txt", ALLRESULTSDIR, dataset)))?;
        result_file.write_all(&results.stdout)?;
    }
    Ok(())
}