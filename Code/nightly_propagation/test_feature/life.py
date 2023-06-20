import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
import psycopg2 as pg
from matplotlib.lines import Line2D

# active accepted removed imcomlete unknown
# blue   green    red     yellow    grey
# 0      1         2        3         4

conn = pg.connect(database='crates', user='postgres', password='postgres')
color = ["cornflower blue", "light green", "coral", "khaki", "light grey"]
cmap = sns.xkcd_palette(color)
column = ["1.%s.0" % i for i in range(0, 68)]

def fetch_lifetime() -> pd.DataFrame:
    cur = conn.cursor()
    # cur.execute('select * from feature_timeline')
    cur.execute('SELECT * FROM feature_timeline WHERE name in (SELECT name from feature_abnormal) ORDER BY "v1_60_0", "v1_50_0", "v1_40_0", "v1_30_0", "v1_20_0", "v1_10_0", "v1_0_0"')
    # cur.execute('SELECT * FROM feature_timeline ORDER BY "v1_60_0", "v1_50_0", "v1_40_0", "v1_30_0", "v1_20_0", "v1_10_0", "v1_0_0"')
    rows = np.array(cur.fetchall())
    lifetime = rows[:, 1:]
    index = rows[:, 0]
    df = pd.DataFrame(data = lifetime, columns=column, index=index).applymap(lambda x: parse(x))
    return df


def parse(s: str) -> int:
    if s == "active":
        return 0
    elif s == "accepted":
        return 1
    elif s == "removed":
        return 2
    elif s == "incomplete":
        return 3
    else:
        return 4 


df = fetch_lifetime()
# print(df.head())

plt.figure(figsize=(8, 4))
custom_lines = [Line2D([0], [0], color=cmap[0], lw=4),
                Line2D([0], [0], color=cmap[1], lw=4),
                Line2D([0], [0], color=cmap[2], lw=4),
                Line2D([0], [0], color=cmap[3], lw=4),
                Line2D([0], [0], color=cmap[4], lw=4)]
plt.legend(custom_lines, ['active', 'accepted', 'removed', 'incomplete', 'unknown'], loc='center right')
plt.subplots_adjust(bottom=0.15, top=0.995, left=0.005, right=1)
sns.heatmap(data = df, cmap=cmap, yticklabels=[], cbar=False)
plt.savefig('figure.pdf', dpi=400, format='pdf')
plt.show()

# print(sns.xkcd_rgb)