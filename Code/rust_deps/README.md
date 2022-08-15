**Set up local registry**

0. Build your Crates.io DB

Get to website `crates.io/data-access` and follows Step 2. The README.md in data will tell you how to set up your DB.

1. Clone remote registry

```bash
git clone https://github.com/rust-lang/crates.io-index
```

And you should change the git commits date according to your DB, in file `metadata.json` of your db-dump file. This makes the index corresponding to your DB.

2. Change .cargo/config.toml

```
[net]
git-fetch-with-cli = true

[source.mirror]
registry = "file:///absolute/path/to/crates.io-index/dir"

[source.crates-io]
replace-with = "mirror"
```

3. Clear your Cache

If you have run this tool, there will be multiple `dep*.toml` file and `job*`  directory, you should delete them all before you start your process.

4. Clear your built tables

If you have run this tool, there will be extra tables in DB which is old. You should drop all these tables.