
# Rust Ecosystem Dependency Graph Generator

### Build projects


1. Change `registry` path in `./.cargo/config.toml` to absolute path to your `crates.io-index`. It should be in the root directory of `Cargo-Ecosystem-Monitor`. If you are using our dockerfile for replication, the default `config.toml` file has been set to the right value. You can ignore this step.

```
[net]
git-fetch-with-cli = true

[source.mirror]
registry = "file:///absolute/path/to/crates.io-index/dir"

[source.crates-io]
replace-with = "mirror"
```


2. Run `cargo run`!

It's OK if you cancel the running process. You can run it again. You won't lost results we have generated. The resolution process will continue.

Be sure that you have correctly set up your database according to our guide in the project root directory. Moreover, if you can't connect to the database, there might be sth wrong with the connection, including port(sometimes 5432, and sometimes 5434) and else.

If you have changed the registry or other ecosystem metadata, you should clear related cache info. In the replication process, this will not happen.


1. Clear your Cache

If you have run this tool, there will be multiple `dep*.toml` file and `job*`  directory, you should delete them all before you start your process.

2. Clear your built tables

If you have run this tool, there will be extra tables in DB which are old. You should drop all these tables.


### Architecture

We separate several threads to resolve crates. The main thread will:

1. Find max crate_id that resolved now, and plus one, which makes it possible that max resolved crate is not fully resolved.
2. Create resolution threads first. The threads will catch panic and store resolution errors. During the resolution process, the threads will wait for the main thread to send `version_info` vector that represents unresolved crates.
3. Find unresolved crates, and pack it into vector, send to resolution threads.
4. Last, re-resolve all unresolved crates due to process breaking. It will get all unresolved crates and build a cache table to store them. Then, get from cache table, pack it into vector and send to resolution threads.

The main process can be conclude into these process:

1. Create virtual env by creating toml file. The toml file contains no feature.
2. Pre-resolve: Resolve current crate. And find all `features` including user-defined and optional dependency from resolution results.
3. Double-resolve: Creating toml file with all features on. Then resolve it.
4. Formatting Resolve and store into DB


### Known Issue

The memory usage will continue rising when running the program, and the program usually not exits as some packages block the processing in the end. You can cancle the program and rerun it to make everything right until almost all packages in the ecosystem have been resolved.