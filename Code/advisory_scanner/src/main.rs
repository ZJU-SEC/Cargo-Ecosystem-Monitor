use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use json;
use std::sync::{Arc, Mutex};
use postgres::{Client, NoTls};
use semver::{VersionReq, Version};

fn main() {

    let conn = Arc::new(Mutex::new(
        Client::connect(
            "host=localhost dbname=crates user=postgres password=postgres",
            NoTls,
        )
        .unwrap(),
    ));
    // If the table already exists, should PANIC!
    // You should drop your table first to make sure the data processed is correct.
    // conn.lock()
    //     .unwrap()
    //     .query(
    //         r#"CREATE TABLE advisory(  
    //             version_id INTEGER,
    //             categories VARCHAR,
    //             UNIQUE(version_id, categories)
    //         );"#,
    //         &[],
    //     )
    //     .unwrap();
    // Github Advisory DB has no advisory categories, but Rustsec has.
    // run_analyze(Arc::clone(&conn), "github_advisory_202203.json", false);
    // run_analyze(Arc::clone(&conn), "rustsec_advisory_202203.json", true);

    run_summary(Arc::clone(&conn));
    
    // println!("parse[0][schema_version]: {:#?}", parsed["0"]["schema_version"]);
}


fn run_summary(conn: Arc<Mutex<Client>>){
    let categories = [
        "memory-corruption",
        "thread-safety",
        "memory-exposure",
        "denial-of-service",
        "crypto-failure",
        "code-execution",
        "format-injection",
        "file-disclosure",
        "privilege-escalation",
    ];

    for category in categories {
        let query_version_count = format!(
            "SELECT COUNT(DISTINCT version_id) FROM advisory WHERE categories like '%{}%';"
            , category
        );
        let query_propagation = format!(
            "SELECT COUNT(DISTINCT version_from) FROM dep_version 
            WHERE version_to IN (SELECT DISTINCT version_id FROM advisory WHERE categories like '%{}%');"
            , category
        );
        let data_version_count: i64 = conn.lock().unwrap().query(&query_version_count, &[]).unwrap().first().unwrap().get(0);
        let data_propagation:i64 = conn.lock().unwrap().query(&query_propagation, &[]).unwrap().first().unwrap().get(0);
        println!("{} : version_count:{}, query_propagation:{}", category, data_version_count, data_propagation);
    }
    let query_version_count = format!(
        "SELECT COUNT(DISTINCT version_id) FROM advisory;"
    );
    let query_propagation = format!(
        "SELECT COUNT(DISTINCT version_from) FROM dep_version 
        WHERE version_to IN (SELECT DISTINCT version_id FROM advisory);"
    );
    let data_version_count: i64 = conn.lock().unwrap().query(&query_version_count, &[]).unwrap().first().unwrap().get(0);
    let data_propagation:i64 = conn.lock().unwrap().query(&query_propagation, &[]).unwrap().first().unwrap().get(0);
    println!("{} : version_count:{}, query_propagation:{}", "Total", data_version_count, data_propagation);
}

/// Analyze Advisory json file
/// @arg: file: File Path Str, has_categories: Is advisory json file contains categories 
fn run_analyze(conn: Arc<Mutex<Client>>, file: &str, has_categories: bool){
    // Create a path to the desired file
    let path = Path::new(file);
    let display = path.display();

    // Open the path in read-only mode, returns `io::Result<File>`
    let mut file = match File::open(&path) {
        Err(why) => panic!("couldn't open {}: {}", display, why),
        Ok(file) => file,
    };

    // Read the file contents into a string, returns `io::Result<usize>`
    let mut s = String::new();
    if let Err(why) = file.read_to_string(&mut s) {
        panic!("couldn't read {}: {}", display, why);
    }
    let parsed = json::parse(&s).unwrap();

    // Test output string
    // println!("parse.len(): {:?}", parsed.len());
    // println!("parse[0]: {:?}", parsed["0"]);
    // println!("parse[0][affected]: {:?}", parsed["0"]["affected"]);
    // println!("parse[0][affected].len(): {:?}", parsed["0"]["affected"].len());
    // println!("parse[0][affected][0]: {:?}", parsed["0"]["affected"][0]);
    // println!("parse[0][affected][0][package]: {:?}", parsed["0"]["affected"][0]["package"]);
    // println!("parse[0][affected][0][package][name]: {:?}", parsed["0"]["affected"][0]["package"]["name"]);
    // println!("parse[0][affected][0][package][name].as_str().unwrap(): {:?}", parsed["0"]["affected"][0]["package"]["name"].as_str().unwrap());
    // println!("parse[0][affected][0][ranges]: {:?}", parsed["0"]["affected"][0]["ranges"]);
    // println!("parse[0][affected][0][ranges][0]: {:?}", parsed["0"]["affected"][0]["ranges"][0]);
    // println!("parse[0][affected][0][ranges][0][events]: {:?}", parsed["0"]["affected"][0]["ranges"][0]["events"]);
    // println!("parse[0][affected][0][ranges][0][events][0][>=]: {:?}", parsed["0"]["affected"][0]["ranges"][0]["events"][0][">="]);
    // println!("parse[0][affected][0][ranges][0][events][0][<]: {:?}", parsed["0"]["affected"][0]["ranges"][0]["events"][0]["<"]);

    // println!("parse[0][affected][0][database_specific]: {:?}", parsed["2"]["affected"][0]["database_specific"]);
    // println!("parse[0][affected][0][database_specific][categories]: {}", parsed["2"]["affected"][0]["database_specific"]["categories"].dump());

    // For every advisory
    let len_i = parsed.len();
    for i in 0..len_i{
        // For every crate in the advisory
        let len_j = parsed[format!("{}",i)]["affected"].len();
        for j in 0..len_j {
            let pkg_name = parsed[format!("{}",i)]["affected"][j]["package"]["name"].as_str().unwrap();
            let advisory_categories:String = if has_categories {
                parsed[format!("{}",i)]["affected"][j]["database_specific"]["categories"].dump()
            }
            else {
                "[]".to_string()
            };
            // For every version rang in the crate
            let len_k = parsed[format!("{}",i)]["affected"][j]["ranges"].len();
            for k in 0..len_k{
                let len_z = parsed[format!("{}",i)]["affected"][j]["ranges"][k]["events"].len();
                for z in 0..len_z{
                    let less = parsed[format!("{}",i)]["affected"][j]["ranges"][k]["events"][z]["<"].as_str();
                    let geq = parsed[format!("{}",i)]["affected"][j]["ranges"][k]["events"][z][">="].as_str();
                    process_advisory(Arc::clone(&conn), less, geq, pkg_name, &advisory_categories);
                }
            }
            
        }

    }
}


/// We assume that arg "geq" is always valid.
fn process_advisory(
    conn: Arc<Mutex<Client>>, 
    less: Option<&str>, 
    geq: Option<&str>, 
    pkg_name: &str,
    advisory_categories: &String
){
    let mut req_str = String::from(">=");
    req_str.push_str(geq.unwrap());
    if let Some(less) = less{
        req_str.push_str(", <");
        req_str.push_str(less);
    }
    let req = VersionReq::parse(&req_str).unwrap();

    let query = format!(
        "SELECT versions.id, num FROM versions INNER JOIN crates 
        ON crate_id=crates.id WHERE name = '{}';", pkg_name
    );

    let data = conn.lock().unwrap().query(&query, &[]).unwrap();

    for row in data {
        let ver_id:i32 = row.get(0);
        let ver_str: &str = row.get(1);
        let ver = Version::parse(ver_str).unwrap();
        if req.matches(&ver){
            let mut query = String::from("INSERT INTO advisory VALUES");
            query.push_str(&format!(
                "({},'{}');",
                ver_id, advisory_categories
            ));
            conn.lock().unwrap().query(&query, &[]).unwrap_or_default();
            // println!("query:{}", query);
            // println!("match: req: {}:{:?}, pkgver:{}", pkg_name, req_str, ver_str);
        }
    }
}