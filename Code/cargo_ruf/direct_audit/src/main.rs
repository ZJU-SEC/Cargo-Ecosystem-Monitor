use std::sync::{Arc, Mutex};

use postgres::{Client, NoTls};
use semver::{Version, VersionReq};

#[derive(Debug, PartialEq, Eq, Hash)]
enum Status {
    Notusable,
    Unstable,
    Stable,
}

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
        .execute(
            "CREATE TABLE IF NOT EXISTS direct_audit_result (
                    version_id INTEGER PRIMARY KEY,
                    fix_status VARCHAR,
                    fix_point VARCHAR
                )",
            &[],
        )
        .unwrap();

    let target_crates = get_target_crates(conn.clone());

    // Here we deal with them one by one
    for one in target_crates {
        println!("Processing crate: {}", one);
        let memdb = match setup_memdb(conn.clone(), &one) {
            Ok(memdb) => memdb,
            Err(e) => {
                eprintln!("Error: {}", e);
                continue;
            }
        };

        // Check issue verions and try find usable
        for (id, ver, _) in memdb
            .iter()
            .filter(|(_, _, status)| *status == Status::Notusable)
        {
            let req = VersionReq::parse(&format!("^{}", ver.to_string())).unwrap();

            let stables = memdb
                .iter()
                .filter(|(_, ver, status)| req.matches(ver) && *status == Status::Stable)
                .map(|(_, ver, _)| ver);

            let unstables = memdb
                .iter()
                .filter(|(_, ver, status)| req.matches(ver) && *status == Status::Unstable)
                .map(|(_, ver, _)| ver);

            let (fix_status, fix_point) = if stables.clone().count() != 0 {
                ("stable", stables.max().unwrap().to_string())
            } else if unstables.clone().count() != 0 {
                ("unstable", unstables.max().unwrap().to_string())
            } else {
                ("fail", "fail".to_string())
            };

            conn.lock().unwrap().execute(
                "INSERT INTO direct_audit_result (version_id, fix_status, fix_point) VALUES ($1, $2, $3)",
                &[&id, &fix_status, &fix_point],
            ).unwrap();
        }
    }
}

/// We setup a hashmap recording the ruf status of each verion in the given crate.
fn setup_memdb(
    conn: Arc<Mutex<Client>>,
    name: &str,
) -> Result<Vec<(i32, Version, Status)>, Box<dyn std::error::Error>> {
    let versions = get_crate_versions(conn.clone(), name)?;
    let mut memdb = Vec::new();

    for (version_id, ver) in versions {
        let rows = conn.lock().unwrap().query(
            "SELECT status FROM version_feature_ori INNER JOIN feature_status ON name = feature WHERE id = $1",
            &[&version_id],
        ).unwrap();

        let mut has_incomplete_or_active = false;
        let mut has_removed_or_unknown = false;
        let mut is_empty = true;

        for row in rows {
            is_empty = false;
            let status: String = row.get("status");
            match status.as_str() {
                "accepted" => {}
                "incomplete" | "active" => has_incomplete_or_active = true,
                "removed" | "unknown" => has_removed_or_unknown = true,
                _ => return Err(format!("unknown status on {name}@{ver}: {status}").into()),
            }
        }

        let status = if is_empty || !has_incomplete_or_active && !has_removed_or_unknown {
            Status::Stable
        } else if has_incomplete_or_active && !has_removed_or_unknown {
            Status::Unstable
        } else {
            Status::Notusable
        };

        memdb.push((version_id, ver, status));
    }

    Ok(memdb)
}

/// Get direct version's crate name. We will resolve them on crates level.
fn get_target_crates(conn: Arc<Mutex<Client>>) -> Vec<String> {
    let mut target_crates = Vec::new();
    let rows = conn
        .lock()
        .unwrap()
        .query(
            "SELECT DISTINCT vw.name
                FROM versions_with_name vw
                JOIN (
                    SELECT id
                    FROM version_feature_ori vfo
                    INNER JOIN feature_status fs ON vfo.feature = fs.name
                    WHERE fs.status IN ('removed', 'unknown')
                    GROUP BY id
                ) tmp ON vw.id = tmp.id",
            &[],
        )
        .unwrap();
    for row in rows {
        target_crates.push(row.get(0));
    }
    target_crates
}

// Get all versions of a given crate.
fn get_crate_versions(
    conn: Arc<Mutex<Client>>,
    name: &str,
) -> Result<Vec<(i32, Version)>, Box<dyn std::error::Error>> {
    let mut crate_versions = Vec::new();
    let rows = conn
        .lock()
        .unwrap()
        .query(
            "SELECT id, num FROM versions_with_name WHERE name = $1",
            &[&name],
        )
        .unwrap();
    for row in rows {
        let id: i32 = row.get(0);
        let ver: String = row.get(1);
        crate_versions.push((
            id,
            Version::parse(&ver)
                .map_err(|e| format!("parsing version {name}@{ver} failed: {e}"))?,
        ));
    }
    Ok(crate_versions)
}
