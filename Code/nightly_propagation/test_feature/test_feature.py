import psycopg2
import subprocess


def do_test(feat_name):
    file = open("./do_test.rs", "w+")
    file.write("#![feature(%s)]\nfn main(){}" % feat_name)
    file.close()

    res = subprocess.run("rustc do_test.rs",
                         capture_output=True, shell=True)

    if len(res.stderr) == 0:
        if res.returncode == 0:
            return "ok", ""
        else:
            return "others", "strange status"
    else:
        if res.stderr.find(b"has been stable") != -1:
            return "ok", str(res.stderr.split(b"\n")[0], encoding='utf-8')
        elif res.stderr.find(b"unknown feature") != -1:
            return "unknown", str(res.stderr.split(b"\n")[0], encoding='utf-8')
        elif res.stderr.find(b"has been removed") != -1:
            return "removed", str(res.stderr.split(b"\n")[0], encoding='utf-8')
        elif res.stderr.find(b"incomplete") != -1:
            return "incomplete", str(res.stderr.split(b"\n")[0], encoding='utf-8')
        else:
            return "other", str(res.stderr.split(b"\n")[0], encoding='utf-8')


conn = psycopg2.connect(
    database="crates", user="postgres", password="postgres")
cur = conn.cursor()

cur.execute('''CREATE TABLE IF NOT EXISTS public.feature_list (
    id serial,
    name VARCHAR(40) PRIMARY KEY,
    status VARCHAR(20),
    info varchar(200)
)''')


cur.execute("SELECT DISTINCT feature FROM version_feature")
for row in cur.fetchall():
    feat_name = row[0]
    status, info = do_test(feat_name)

    cur.execute("INSERT INTO feature_list (name, status, info) VALUES('%s', '%s', '%s')" %
                (feat_name, status, info))

    print("%s %s" % (feat_name, status))
conn.commit()
conn.close()