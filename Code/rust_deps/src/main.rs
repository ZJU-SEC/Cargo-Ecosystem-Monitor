use postgres::{Client, NoTls};
use util::*;

mod util;

fn main() {
    let mut conn = Client::connect(
        "host=localhost dbname=crates user=postgres password=postgres",
        NoTls,
    )
    .unwrap();
    conn.query(
        r#"CREATE TABLE dep_version(
                        version_from INT,
                        version_to INT,
                        dep_level INT,
                        UNIQUE(version_from, version_to, dep_level))"#,
        &[],
    )
    .unwrap_or_default();

    resolve_store_deps_of_version(&mut conn, 505768);
}