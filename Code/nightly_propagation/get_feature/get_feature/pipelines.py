# Define your item pipelines here
#
# Don't forget to add your pipeline to the ITEM_PIPELINES setting
# See: https://docs.scrapy.org/en/latest/topics/item-pipeline.html


# useful for handling different item types with a single interface
from gettext import find
from sys import stderr
import psycopg2
import subprocess


class FeaturePipeline:
    def __init__(self) -> None:
        self.all_features = []
        self.conn = psycopg2.connect(
            database="crates", user="postgres", password="postgres")

    def open_spider(self, spider):
        cur = self.conn.cursor()

        cur.execute('''CREATE TABLE IF NOT EXISTS public.feature_list (
            name VARCHAR(40) PRIMARY KEY,
            status VARCHAR(20),
            info varchar(200)
        )''')

        self.conn.commit()

    def close_spider(self, spider):
        cur = self.conn.cursor()

        for feat_name in self.all_features:
            status, info = do_test(feat_name)

            cur.execute("INSERT INTO feature_list VALUES ('%s', '%s', '%s')" %
                        (feat_name, status, info))

            print("%s %s" % (feat_name, status))

        self.conn.commit()
        self.conn.close()

    def process_item(self, item, spider):
        self.all_features.append(item['name'])
        return item


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
        if res.stderr.find(b"unknown feature") != -1:
            return "unknown", res.stderr
        elif res.stderr.find(b"incomplete") != -1:
            return "incomplete", res.stderr
        else:
            return "others", res.stderr
