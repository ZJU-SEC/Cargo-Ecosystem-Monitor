import re
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
import psycopg2 as pg
from matplotlib.lines import Line2D

# active accepted removed imcomlete unknown
# blue   green    red     yellow    grey
# 0      1         2        3         4

conn = pg.connect(database='crates', user='postgres', password='postgres', port='5434')

cur = conn.cursor()
cur.execute('SELECT * FROM feature_timeline')
rows = np.array(cur.fetchall())
# print(rows.shape[1])

lifetime = rows[:, 1:]
ruf_name = rows[:, 0]
RUSTC_VER_NUM = lifetime.shape[1]
RUF_SIZE = lifetime.shape[0]
lines = ""
for i in  range(0, RUF_SIZE):
    line = "lifetime.insert(" +"\"" + ruf_name[i] + "\",[" 
    for ruf_status in lifetime[i]:
        line += "\"" + str(ruf_status) + "\","
    line += "]);\n"
    lines += line

file = r"""// This file is auto-generated. Don't modify.
use std::collections::HashMap;
pub const RUSTC_VER_NUM:usize = """ + str(RUSTC_VER_NUM) + r""";
pub fn get_lifetime_raw () -> HashMap<&'static str, [&'static str; RUSTC_VER_NUM]> { 
let mut lifetime = HashMap::new();
""" + lines +"""lifetime }"""
fo = open("./src/lifetime.rs", "w")
fo.write(file)
fo.close()


