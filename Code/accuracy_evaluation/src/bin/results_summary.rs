use std::fs::{self, File};
use std::io::BufReader;
use std::io::prelude::*;

use regex::Regex;

const ACCURACY_BASE:i32 = 90;

const RESULTSDIR:&str = "./output/results";
fn main() -> std::io::Result<()> {
    println!("This is results_summary");
    // Match results regex
    let cargotree_crates_re = Regex::new("cargotree_crates_num = [0-9]+").unwrap();
    let pipeline_crates_re  = Regex::new("pipeline_crates_num = [0-9]+").unwrap();
    let overresolve_re      = Regex::new("overresolve_dep = [0-9]+").unwrap();
    let right_re            = Regex::new("right_dep = [0-9]+").unwrap();
    let wrong_re            = Regex::new("wrong_dep = [0-9]+").unwrap();
    let missing_re          = Regex::new("missing_dep = [0-9]+").unwrap();
    let mut cargotree_crates_num    = 0;
    let mut pipeline_crates_num     = 0;
    let mut overresolve_dep         = 0;
    let mut right_dep               = 0;
    let mut wrong_dep               = 0;
    let mut missing_dep             = 0;
    // Read every results
    let paths = fs::read_dir(RESULTSDIR).unwrap();
    for path_dir in paths {
        let path = path_dir.unwrap().path();
        // Read each file
        // println!("Name: {}", path.display());
        let display = path.display();   
        let file = match File::open(&path) {
            Err(why) => panic!("couldn't create {}: {}", display, why),
            Ok(file) => file,
        };
        let mut buf_reader = BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents)?;
        // Match data
        let cap_cargotree_crates_num = cargotree_crates_re.captures(&contents).unwrap();
        let cap_pipeline_crates_num  = pipeline_crates_re.captures(&contents).unwrap();
        let cap_overresolve_dep      = overresolve_re.captures(&contents).unwrap();
        let cap_right_dep            = right_re.captures(&contents).unwrap();
        let cap_wrong_dep            = wrong_re.captures(&contents).unwrap();
        let cap_missing_dep          = missing_re.captures(&contents).unwrap();
        let cargotree_crates_num_single = &cap_cargotree_crates_num [0].replace("cargotree_crates_num = ", "").parse::<i32>().unwrap();
        let pipeline_crates_num_single  = &cap_pipeline_crates_num  [0].replace("pipeline_crates_num = ", "").parse::<i32>().unwrap();
        let overresolve_dep_single      = &cap_overresolve_dep      [0].replace("overresolve_dep = ", "").parse::<i32>().unwrap();
        let right_dep_single            = &cap_right_dep            [0].replace("right_dep = ", "").parse::<i32>().unwrap();
        let wrong_dep_single            = &cap_wrong_dep            [0].replace("wrong_dep = ", "").parse::<i32>().unwrap();
        let missing_dep_single          = &cap_missing_dep          [0].replace("missing_dep = ", "").parse::<i32>().unwrap();
        // Alarm
        if *cargotree_crates_num_single == 0 {
            println!("Cargo tree resolve failure Alarm: {}", display);
        }
        else if  *pipeline_crates_num_single == 0 {
            println!("Pipeline resolve failure Alarm: {}", display);
            continue;
        }
        else if (right_dep_single * 100) < (ACCURACY_BASE * cargotree_crates_num_single) {
            println!("Accuracy Alarm: {}", display);
        }
        cargotree_crates_num    += cargotree_crates_num_single;
        pipeline_crates_num     += pipeline_crates_num_single ;
        overresolve_dep         += overresolve_dep_single     ;
        right_dep               += right_dep_single           ;
        wrong_dep               += wrong_dep_single           ;
        missing_dep             += missing_dep_single         ;
    }
    println!("cargotree_crates_num = {}", cargotree_crates_num );
    println!("pipeline_crates_num  = {}", pipeline_crates_num  );
    println!("overresolve_dep      = {}", overresolve_dep      );
    println!("right_dep            = {}", right_dep            );
    println!("wrong_dep            = {}", wrong_dep            );
    println!("missing_dep          = {}", missing_dep          );
    Ok(())
}