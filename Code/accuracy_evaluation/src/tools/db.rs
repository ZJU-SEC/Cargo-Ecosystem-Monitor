use std::collections::{HashSet, HashMap};
use std::sync::{Arc, Mutex};
use rand::distributions::{Distribution, Uniform};

use postgres::{Client};

use crate::CRATES_NUM;

#[derive(Debug)]
pub struct CrateInfo{
    pub crate_id: i32,
    pub version_id: i32,
    pub name: String,
    pub version_num: String,
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
                    SELECT crate_id, version_id, name, version_num, 'unevaluated' as status
                    FROM most_dep_version INNER JOIN crate_newestversion
                    ON version_id = newest_version_id WHERE yanked = false ORDER BY deps desc LIMIT {}",
                 CRATES_NUM),
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
            status: ver.get(4),
        }
    ).collect()
}


pub fn find_unevaluated_crates_rand(conn: Arc<Mutex<Client>>) -> Vec<CrateInfo> {
    conn.lock()
        .unwrap()
        .query(
            r#"CREATE TABLE IF NOT EXISTS public.accuracy_evaluation_status
            (
                crate_id INT,
                version_id INT,
                name VARCHAR,
                version_num VARCHAR,
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
        println!("Start to build accuracy evaluation random test dataset");
        // Empty: Select Random Crates to build table
        conn.lock()
            .unwrap()
            .query(
                r#"DROP TABLE IF EXISTS public.tmp_accuracy_evaluation_rand"#,
                &[],
            )
            .unwrap();
        conn.lock()
            .unwrap()
            .query(
                r#"CREATE TABLE public.tmp_accuracy_evaluation_rand
                (
                    crate_id INT
                )"#,
                &[],
            )
            .unwrap();

        // Find max crate id
        let max_crate_id:i32 = conn.lock().unwrap()
            .query("
                SELECT MAX(crate_id) FROM crate_newestversion WHERE yanked=false",
                &[],
            ).unwrap().first().unwrap().get(0);
        let mut rng = rand::thread_rng();
        let rand_uniform = Uniform::from(1..(max_crate_id+1));
        let mut size = 0;
        let mut selected_crate = HashSet::new();
        loop {
            let rand_crate_id = rand_uniform.sample(&mut rng);
            // Crate exists and not yanked.
            if conn.lock().unwrap()
                .query(&format!(
                    "SELECT * FROM crate_newestversion WHERE yanked=false AND crate_id = {}"
                    , rand_crate_id),
                    &[],
                ).unwrap().first().is_none(){
                continue;
            }
            // Not selected.
            if selected_crate.contains(&rand_crate_id){
                continue;
            }
            // Then select this.
            conn.lock().unwrap()
                .query(&format!(
                    "INSERT INTO tmp_accuracy_evaluation_rand VALUES ({})"
                    , rand_crate_id),
                    &[],
                ).unwrap();
            selected_crate.insert(rand_crate_id);
            size += 1;
            if size == CRATES_NUM {
                break;
            }
        }

        // Build Table
        conn.lock().unwrap()
            .query(
                "INSERT INTO public.accuracy_evaluation_status
                    SELECT tmp_accuracy_evaluation_rand.crate_id, newest_version_id as version_id, name, version_num, 'unevaluated' as status
                    FROM tmp_accuracy_evaluation_rand INNER JOIN crate_newestversion
                    ON tmp_accuracy_evaluation_rand.crate_id = crate_newestversion.crate_id",
                &[],
            ).unwrap();
    }
    
    // Get unevaluated crates
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
            status: ver.get(4),
        }
    ).collect()
}

pub fn find_unevaluated_crates_hot(conn: Arc<Mutex<Client>>) -> Vec<CrateInfo> {
    conn.lock()
        .unwrap()
        .query(
            r#"CREATE TABLE IF NOT EXISTS public.accuracy_evaluation_status
            (
                crate_id INT,
                version_id INT,
                name VARCHAR,
                version_num VARCHAR,
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
                INSERT INTO public.accuracy_evaluation_status 
                    SELECT crate_id, newest_version_id as version_id, crates.name, version_num, 'unevaluated' as status
                    FROM crates INNER JOIN crate_newestversion
                    ON id = crate_id WHERE yanked = false ORDER BY downloads desc LIMIT {}",
                 CRATES_NUM),
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
            status: ver.get(4),
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
                status: ver.get(4),
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

#[test]
fn rnd_test(){
    let max_crate_id = 100;
    let mut rng = rand::thread_rng();
    let rand_uniform = Uniform::from(1..(max_crate_id+1));
    loop {
        let rand_crate_id = rand_uniform.sample(&mut rng);
        println!("rand_crate_id: {}", rand_crate_id);
        if rand_crate_id == 100{
            break;
        }
    }
}