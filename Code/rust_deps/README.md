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

### Architecture

We seperate several threads to resolve crates. The main thread will:

1. Find max crate_id that resolved now, and plus one, which makes it possible that max resolved crate is not fully resolved.
2. Create resolution threads first. The threads will catch panic and store resolution errors. During the resolution process, the threads will wait for the main thread to send `version_info` vector that represents unresolved crates.
3. Find unresolved crates, and pack it into vector, send to resolution threads.
4. Last, re-resolve all unresolved crates due to process breaking. It will get all unresolved crates and build a cache table to store them. Then, get from cache table, pack it into vector and send to resolution threads.

The main process can be conclude into these process:

1. Create virtual env by creating toml file. The toml file contains no feature.
2. Pre-resolve: Resolve current crate. And find all `features` including user-defined and optional dependency from resolution results.
3. Double-resolve: Creating toml file with all features on. Then resolve it.
4. Formatting Resolve and store into DB