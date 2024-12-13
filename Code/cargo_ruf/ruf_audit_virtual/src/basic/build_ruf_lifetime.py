import numpy as np
import psycopg2 as pg
import os

conn = pg.connect(database="crates", user="postgres", password="postgres")
cur = conn.cursor()
cur.execute("SELECT * FROM feature_timeline")
rows = np.array(cur.fetchall())

lifetime = rows[:, 1:]
ruf_name = rows[:, 0]
RUSTC_VER_NUM = lifetime.shape[1]
RUF_SIZE = lifetime.shape[0]
lines = ""
for i in range(0, RUF_SIZE):
    line = "lifetime.insert(" + '"' + ruf_name[i] + '",['
    for ruf_status in lifetime[i]:
        if ruf_status == None:
            ruf_status = "0"
        elif ruf_status == "active":
            ruf_status = "1"
        elif ruf_status == "incomplete":
            ruf_status = "2"
        elif ruf_status == "accepted":
            ruf_status = "3"
        elif ruf_status == "removed":
            ruf_status = "4"
        else:
            raise Exception("Unknown status: " + ruf_status)
        # line += '"' + ruf_status + '",'
        line += ruf_status + ","
    line += "]);\n"
    lines += line


file = (
    r"""// This file is auto-generated. Don't modify.
use fxhash::FxHashMap;
pub const RUSTC_VER_NUM: usize = """
    + str(RUSTC_VER_NUM)
    + r""";
pub fn get_lifetime_raw () -> FxHashMap<&'static str, [u8; RUSTC_VER_NUM]> { 
let mut lifetime =  FxHashMap::default();
"""
    + lines
    + """lifetime }"""
)

current_dir_path = os.path.dirname(os.path.abspath(__file__))
fo = open(current_dir_path + "/src/lifetime.rs", "w")
fo.write(file)
fo.close()
