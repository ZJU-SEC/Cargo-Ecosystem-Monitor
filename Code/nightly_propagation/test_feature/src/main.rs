use postgres::{Client, NoTls};
use std::fs::OpenOptions;
use std::io::Write;
use std::process::Command;
use std::sync::{Arc, Mutex};

fn main() {
    // set up
    let set_up_res = Command::new("sh")
        .arg("src/setup.sh")
        .output()
        .expect("failed to execute process");
    if !set_up_res.status.success() {
        panic!("failed to set up");
    }

    let conn = Arc::new(Mutex::new(
        Client::connect(
            "host=localhost dbname=crates user=postgres password=postgres",
            NoTls,
        )
        .unwrap(),
    ));

    prebuild(Arc::clone(&conn));

    let features: Vec<String> = conn
        .lock()
        .unwrap()
        .query("SELECT DISTINCT feature FROM version_feature;", &[])
        .unwrap()
        .into_iter()
        .map(|feat| feat.get(0))
        .collect();

    for feat in features {
        let (status, info) = do_test(&feat);
        conn.lock()
            .unwrap()
            .query(
                &format!(
                    "INSERT INTO feature_status VALUES('{}', '{}', '{}')",
                    feat, status, info
                ),
                &[],
            )
            .unwrap();

        println!("{} {}", feat, status);
    }
}

fn prebuild(conn: Arc<Mutex<Client>>) {
    conn.lock()
        .unwrap()
        .query(
            r#"CREATE TABLE IF NOT EXISTS public.feature_status (
        name VARCHAR,
        status VARCHAR,
        info VARCHAR)"#,
            &[],
        )
        .unwrap();
}

fn do_test(feature: &str) -> (String, String) {
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open("do_test.rs")
        .expect("Open file fails");

    let buf = format!("#![feature({})]\nfn main() {{}}", feature);
    file.write_all(&buf.as_bytes()).expect("Write file fails");

    let res = Command::new("rustc")
        .arg("do_test.rs")
        .output()
        .expect("failed to execute process");

    let stderr = String::from_utf8(res.stderr).expect("resolve rustc result fails");

    if stderr.len() == 0 {
        if res.status.success() {
            return ("ok".to_string(), "".to_string());
        } else {
            return ("others".to_string(), "unexpected fails".to_string());
        }
    } else {
        if stderr.contains("has been stable") {
            return ("stablized".to_string(), stderr);
        } else if stderr.contains("unknown feature") {
            return ("unknown".to_string(), stderr);
        } else if stderr.contains("has been removed") {
            return ("removed".to_string(), stderr);
        } else if stderr.contains("incomplete") {
            return ("incomplete".to_string(), stderr);
        } else {
            return ("others".to_string(), stderr);
        }
    }
}
