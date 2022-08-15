use std::collections::{HashSet, HashMap};
use std::sync::{Arc, Mutex};

use postgres::{Client};

use crate::CRATES_NUM;

#[derive(Debug)]
pub struct CrateInfo{
    pub crate_id: i32,
    pub version_id: i32,
    pub name: String,
    pub version_num: String,
    pub dep: i32,
    pub status: String,
}

pub fn store_fails_info(conn: Arc<Mutex<Client>>, crate_id: i32) {
    conn.lock()
        .unwrap()
        .query(
            &format!(
                "UPDATE accuracy_evaluation_status SET status = 'fails' WHERE crate_id = '{}';",
                crate_id
            ),
            &[],
        )
        .expect("Fatal error, store info fails!");
}

pub fn find_unevaluated_crates(conn: Arc<Mutex<Client>>) -> Vec<CrateInfo> {
    conn.lock()
        .unwrap()
        .query(
            r#"CREATE TABLE IF NOT EXISTS public.accuracy_evaluation_status
            (
                crate_id INT,
                version_id INT,
                name VARCHAR,
                version_num VARCHAR,
                deps INT,
                status VARCHAR
            )"#,
            &[],
        )
        .unwrap();
    // Check if table is empty
    if conn.lock().unwrap().query(
            "SELECT * FROM accuracy_evaluation_status LIMIT 1",
            &[],
        ).unwrap().first().is_none()
    {
        // Empty: Select top crates with most direct dependency
        conn.lock().unwrap()
            .query(
                &format!("
                WITH most_dep_version AS
                (SELECT version_id, COUNT(crate_id) as deps FROM dependencies GROUP BY version_id)
                INSERT INTO public.accuracy_evaluation_status 
                SELECT crate_id, version_id, name, version_num, deps, 'unevaluated' as status
                FROM most_dep_version INNER JOIN crate_newestversion
                ON version_id = newest_version_id ORDER BY deps desc LIMIT {}", CRATES_NUM),
                &[],
            ).unwrap().first();
    }
    let query = format!(
        "SELECT * FROM accuracy_evaluation_status WHERE status = 'unevaluated'"
    );
    let row = conn.lock().unwrap().query(&query, &[]).unwrap();
    row.iter().map(|ver| 
        CrateInfo{
            crate_id:ver.get(0),
            version_id: ver.get(1),
            name: ver.get(2),
            version_num: ver.get(3),
            dep: ver.get(4),
            status: ver.get(5),
        }
    ).collect()
}

pub fn find_resolved_crates(conn: Arc<Mutex<Client>>) -> Vec<CrateInfo> {
    let query = format!(
        "SELECT * FROM accuracy_evaluation_status WHERE status = 'resolved'"
    );
    if let Ok(row) = conn.lock().unwrap().query(&query, &[]){
        row.iter().map(|ver| 
            CrateInfo{
                crate_id:ver.get(0),
                version_id: ver.get(1),
                name: ver.get(2),
                version_num: ver.get(3),
                dep: ver.get(4),
                status: ver.get(5),
            }
        ).collect()
    }
    else{
        Vec::new()
    }
}

// Get results of our `Cargo Ecosystem Monitor Dependency Resolution Pipeline`
pub fn get_pipeline_results(
    conn: Arc<Mutex<Client>>,
    crate_info: &CrateInfo,
) -> HashMap<String, HashSet<String>>{
    // Data structure of `dependencies`: HashMap<crate_name, HashSet<versions> >
    let mut dependencies:HashMap<String, HashSet<String>> = HashMap::new();
    let query = format!(
        "WITH target_dep AS(
            WITH target_version AS 
            (SELECT distinct version_to FROM dep_version
            WHERE version_from = {})
            SELECT crate_id, num FROM target_version INNER JOIN versions ON version_to = id)
            SELECT name, num FROM target_dep INNER JOIN crates ON crate_id = id ORDER BY num asc
        ", crate_info.version_id
    );
    let row = conn.lock().unwrap().query(&query, &[]).unwrap();
    for ver in row {
        let dep_name = ver.get(0);
        let dep_ver = ver.get(1);
        let crate_name = dependencies.entry(dep_name).or_insert(HashSet::new());
        (*crate_name).insert(dep_ver);
    }
    dependencies
}