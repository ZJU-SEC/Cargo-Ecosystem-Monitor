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
    conn.lock()
        .unwrap()
        .query(
            r#"CREATE TABLE advisory(  
                version_id INTEGER,
                UNIQUE(version_id));"#,
            &[],
        )
        .unwrap_or_default();
    
    run_analyze(Arc::clone(&conn), "rust_advisory_change.json");
    run_analyze(Arc::clone(&conn), "rustsec_ranges_change.json");

    
    // println!("parse[0][schema_version]: {:#?}", parsed["0"]["schema_version"]);
}

fn run_analyze(conn: Arc<Mutex<Client>>, file: &str){
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

    let i = 0;
    let j = 0;
    let len_i = parsed.len();
    for i in 0..len_i{
        let len_j = parsed[format!("{}",i)]["affected"].len();
        for j in 0..len_j {
            let pkg_name = parsed[format!("{}",i)]["affected"][j]["package"]["name"].as_str().unwrap();
            let len_k = parsed[format!("{}",i)]["affected"][j]["ranges"].len();
            for k in 0..len_k{
                let len_z = parsed[format!("{}",i)]["affected"][j]["ranges"][k]["events"].len();
                for z in 0..len_z{
                    let less = parsed[format!("{}",i)]["affected"][j]["ranges"][k]["events"][z]["<"].as_str();
                    let geq = parsed[format!("{}",i)]["affected"][j]["ranges"][k]["events"][z][">="].as_str();
                    process_advisory(Arc::clone(&conn), less, geq, pkg_name);
                }
            }
            
        }

    }
}


/// We assume that arg "geq" is always valid.
fn process_advisory(conn: Arc<Mutex<Client>>, less: Option<&str>, geq: Option<&str>, pkg_name: &str){
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
                "({}),",
                ver_id,
            ));
            query.pop();
            query.push(';');
            conn.lock().unwrap().query(&query, &[]).unwrap_or_default();
            // println!("match: req: {}:{:?}, pkgver:{}", pkg_name, req_str, ver_str);
        }
    }
}