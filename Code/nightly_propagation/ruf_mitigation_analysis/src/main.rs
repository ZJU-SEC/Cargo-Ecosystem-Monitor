use std::collections::{HashSet, HashMap};
use std::sync::{Arc, Mutex};
use std::path::Path;
use std::fs::{create_dir, remove_dir_all, File, OpenOptions};
use std::io::prelude::*;

use RUF_mitigation::{get_ruf_status, get_lifetime};
use lifetime::RUSTC_VER_NUM;
use postgres::{Client, NoTls};
mod lifetime;

const MAX_RUSTC_VERSION:usize = 63; // 1.0.0 -> 1.63.0
const RESULTSFILE:&str = "./mitigation_results.csv";

fn main() {
    let path = Path::new(RESULTSFILE);
    let mut file = File::create(&path).unwrap();
    let line = "ver_id,before_status,after_mitigation,recovery_point\n";
    if let Err(why) = file.write_all(line.as_bytes()) {
        panic!("couldn't write to {}: {}", path.display(), why);
    }


    let conn = Arc::new(Mutex::new(
        Client::connect(
            "host=localhost dbname=crates user=postgres password=postgres",
            NoTls,
        )
        .unwrap(),
    ));
    println!("Build RUF Impact Table (May take minutes)...");
    let ruf_impacts = get_ruf_impact(Arc::clone(&conn));
    let ruf_lifetime = get_lifetime();
    init_mitigation_results_db(Arc::clone(&conn));
    println!("Simulate Mitigation Process...");
    // println!("count {:#?}", ruf_impacts);

    // 1. Newest version status
    let mut count = 0;
    let mut before_count_failure = 0;
    let mut before_count_unstable = 0;
    let mut before_count_stable = 0;
    let mut after_count_failure = 0;
    let mut after_count_unstable = 0;
    let mut after_count_stable = 0;
    for (ver, ruf_impact) in &ruf_impacts {
        count += 1;
        if (count % 10000) == 0 {
            println!("Processing {count}th package version...");
        }  
        // Before Mitigation
        let before_status = get_version_ruf_status(ruf_impact, MAX_RUSTC_VERSION , &ruf_lifetime);
        match before_status {
            "failure"   => before_count_failure += 1,
            "unstable"  => before_count_unstable += 1,
            "stable"    => before_count_stable += 1,
            _ => (),
        };
        // After Mitigation
        let (after_status, recovery_point) = get_version_ruf_status_all(ruf_impact, &ruf_lifetime);
        match after_status {
            "failure"   => after_count_failure += 1,
            "unstable"  => after_count_unstable += 1,
            "stable"    => after_count_stable += 1,
            _ => (),
        };
        // Write detail to result file
        let line = format!("{},{},{},{}\n", *ver, before_status, after_status,recovery_point);
        if let Err(why) = file.write_all(line.as_bytes()) {
            panic!("couldn't write to {}: {}", path.display(), why);
        }
        store_mitigation_results_db(Arc::clone(&conn), *ver, before_status, after_status, recovery_point);
    }
    println!("Count {}"                 , count);
    println!("Newest Version");
    println!("before_count_failure  {}" , before_count_failure);
    println!("before_count_unstable {}" , before_count_unstable);
    println!("before_count_stable   {}" , before_count_stable);
    println!("After Mitigation");
    println!("after_count_failure   {}" , after_count_failure);
    println!("after_count_unstable  {}" , after_count_unstable);
    println!("after_count_stable {  }"  , after_count_stable);



}


/// Pre build
/// Return: ruf impact <id, Vec<RUF>>
fn get_ruf_impact(conn: Arc<Mutex<Client>>) -> HashMap<i32, Vec<String>> {
    conn.lock().unwrap().
        query("DROP TABLE IF EXISTS tmp_ruf_remediation_analysis;", &[]).unwrap();
    conn.lock().unwrap().
        query(r#"CREATE TABLE tmp_ruf_remediation_analysis AS (
            SELECT DISTINCT id, feature FROM version_feature
            WHERE feature != 'no_feature_used'
        );"#, &[]).unwrap();
    conn.lock().unwrap().
        query(r#"INSERT INTO tmp_ruf_remediation_analysis
        SELECT DISTINCT version_from, feature FROM version_feature 
        INNER JOIN dep_version ON version_to=id WHERE conds = '' AND feature IS NOT NULL;"#, &[]).unwrap();
    conn.lock().unwrap().
        query(r#"INSERT INTO tmp_ruf_remediation_analysis
        SELECT  DISTINCT version_from, nightly_feature FROM dep_version_feature;"#, &[]).unwrap();
    let query = format!(
        "SELECT DISTINCT id, feature FROM tmp_ruf_remediation_analysis;"
    );
    let rows = conn.lock().unwrap().query(&query, &[]).unwrap();
    let mut ruf_impact: HashMap<i32, Vec<String>> = HashMap::new();
    for ver in rows{
        let version_id:i32 = ver.get(0);
        let name:&str = ver.get(1);
        let id = ruf_impact.entry(version_id).or_insert(Vec::new());
        (*id).push(name.to_string());
    }
    ruf_impact
}


/// Return the best ruf status through all rustc version where specific package version is using. This represents that the version can be safety recovered.
///     Arg: ruf_impact: Vec<RUF>, rustc_version: Specify rustc_version where RUF is running, lifetime_table: RUF lifetime
///     Return: Worst ruf status. Can be "stable", "unstable" (including "active" and "imcomplete"), "failure" (including "removed" and "unknown").
///             Also, return the recovery point.
fn get_version_ruf_status_all(ruf_impact: &Vec<String>, lifetime_table: &HashMap<&'static str, [&'static str; RUSTC_VER_NUM]> ) -> (&'static str, usize){
    let mut final_status = "failure";
    let mut recovery_point = MAX_RUSTC_VERSION;
    for i in (0..(MAX_RUSTC_VERSION + 1)).rev(){
        let status = get_version_ruf_status(ruf_impact, i, lifetime_table);
        match status {
            "stable" => return ("stable", i),
            "unstable" => {
                if final_status != "unstable" {
                    recovery_point = i;
                }
                final_status = "unstable";
            }
            _ => (),
        };
    }
    (final_status, recovery_point )
}

/// Return the worst ruf status (given rustc version) where specific package version is using. This represents that whether the version can be safety used.
///     Arg: ruf_impact: Vec<RUF>, rustc_version: Specify rustc_version where RUF is running, lifetime_table: RUF lifetime
///     Return: Worst ruf status. Can be "stable", "unstable" (including "active" and "imcomplete"), "failure" (including "removed" and "unknown").
fn get_version_ruf_status(ruf_impact: &Vec<String>, rustc_version:usize, lifetime_table: &HashMap<&'static str, [&'static str; RUSTC_VER_NUM]> ) -> &'static str{
    let mut status = "stable";
    for ruf in ruf_impact{
        if let Some(ruf_status) = get_ruf_status(lifetime_table, ruf.as_str(), rustc_version){
            if ruf_status == "active" || ruf_status == "imcomplete" {
                status = "unstable";
            }
            else if ruf_status == "removed"{
                return "failure";
            }
        }
        else{
            return "failure";
        }
    }
    status
}

fn init_mitigation_results_db(conn: Arc<Mutex<Client>>) {
    conn.lock().unwrap().
        query("DROP TABLE IF EXISTS mitigation_results", &[]).unwrap();
    conn.lock()
        .unwrap()
        .query(
            r#"CREATE TABLE public.mitigation_results
            (
                ver_id INT,
                before_mitigation VARCHAR,
                after_mitigation VARCHAR,
                recovery_point INT
            )"#,
            &[],
        )
        .unwrap();
}

fn store_mitigation_results_db(
    conn: Arc<Mutex<Client>>,
    ver_id: i32,
    before_mitigation: &str,
    after_mitigation: &str,
    recovery_point: usize) 
{
    conn.lock().unwrap()
                .query(&format!(
                    "INSERT INTO mitigation_results VALUES ({},'{}','{}',{})"
                    , ver_id, before_mitigation, after_mitigation, recovery_point),
                    &[],
                ).unwrap();
}