use std::collections::{HashSet, HashMap};
use std::sync::{Arc, Mutex};

use RUF_remediation::{get_ruf_status, get_lifetime};
use lifetime::RUSTC_VER_NUM;
use postgres::{Client, NoTls};
mod lifetime;

const MAX_RUSTC_VERSION:usize = 63; // 1.0.0 -> 1.63.0

fn main() {
    let conn = Arc::new(Mutex::new(
        Client::connect(
            "host=localhost dbname=crates user=postgres password=postgres",
            NoTls,
        )
        .unwrap(),
    ));
    let ruf_impacts = get_ruf_impact(Arc::clone(&conn));
    let ruf_lifetime = get_lifetime();
    // println!("count {:#?}", ruf_impacts);

    // 1. Newest version status
    let mut count = 0;
    let mut count_failure = 0;
    let mut count_unstable = 0;
    let mut count_stable = 0;
    for (_ver, ruf_impact) in &ruf_impacts {
        count += 1;
        let status = get_version_ruf_status(ruf_impact, MAX_RUSTC_VERSION , &ruf_lifetime);
        match status {
            "failure" => count_failure += 1,
            "unstable" => count_unstable += 1,
            "stable" => count_stable += 1,
            _ => (),
        };
        // println!("status: {}, impacts:{:?}", status, ruf_impact);
    }
    println!("Newest_version");
    println!("count {}", count);
    println!("count_failure {}", count_failure);
    println!("count_unstable {}", count_unstable);
    println!("count_stable {}", &count_stable);

    // 2. All possible version status
    let mut count = 0;
    let mut count_failure = 0;
    let mut count_unstable = 0;
    let mut count_stable = 0;
    for (_ver, ruf_impact) in &ruf_impacts {
        count += 1;
        let status = get_version_ruf_status_all(ruf_impact, &ruf_lifetime);
        match status {
            "failure" => count_failure += 1,
            "unstable" => count_unstable += 1,
            "stable" => count_stable += 1,
            _ => (),
        };
        // println!("status: {}, impacts:{:?}", status, ruf_impact);
    }
    println!("All versions");
    println!("count {}", count);
    println!("count_failure {}", count_failure);
    println!("count_unstable {}", count_unstable);
    println!("count_stable {}", &count_stable);
    


}


/// Pre build
/// Return: ruf impact <id, Vec<RUF>>
fn get_ruf_impact(conn: Arc<Mutex<Client>>) -> HashMap<i32, Vec<String>> {
    // conn.lock()
    // .unwrap()
    // .query(
    // r#"DROP TABLE IF EXISTS tmp_ruf_remediation_analysis;
    //     CREATE TABLE tmp_ruf_remediation_analysis AS (
    //         SELECT DISTINCT id, feature FROM version_feature
    //         WHERE feature IS NOT NULL
    //     );
    //     INSERT INTO tmp_ruf_remediation_analysis
    //         SELECT DISTINCT version_from, feature FROM version_feature 
    //         INNER JOIN dep_version ON version_to=id WHERE conds = '' AND feature IS NOT NULL;
    //     INSERT INTO tmp_ruf_remediation_analysis
    //         SELECT  DISTINCT version_from, nightly_feature FROM dep_version_feature;"#,
    // &[],
    // )
    // .unwrap();
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
fn get_version_ruf_status_all(ruf_impact: &Vec<String>, lifetime_table: &HashMap<&'static str, [&'static str; RUSTC_VER_NUM]> ) -> &'static str{
    let mut final_status = "failure";
    for i in 0..(MAX_RUSTC_VERSION + 1){
        let status = get_version_ruf_status(ruf_impact, i, lifetime_table);
        match status {
            "stable" => return "stable",
            "unstable" => final_status = "unstable",
            _ => (),
        };
    }
    final_status
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