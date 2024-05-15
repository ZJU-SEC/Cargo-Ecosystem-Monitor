
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

### Run a single deps

If you want to view dependency trees given a package version, you can run `get_deps`.
```Shell
cargo run --bin get_deps <name> <version_num>
```
The output format follows: `<dep_name>,<version_num>,<dep_depth>`. For example, if we run `cargo run --bin get_deps rand 0.8.5`, the output should be
```Shell
unicode-ident,1.0.3,4
unicode-ident,1.0.3,7
syn,1.0.99,3
cfg-if,1.0.0,2
proc-macro2,1.0.43,3
rand_chacha,0.3.1,1
xxx (Omitted)
```


There is one more command for release usage: 
```Shell
cargo run --bin get_deps release <name> <version_num>
```
This fixes resolution errors caused by new Cargo dependency configurations. We're currently porting this fixed version to our ecosystem resolver to correct errors. We're also expecting Cargo officials to resolve new configurations in their core resolver. In this way, our ecosystem resolver can still work without modification.


If you want to view full dependency info, you can run
```Shell
cargo run --bin get_deps full <name> <version_num>
```
The output format follows: `graph:{ dependenies of each package version} features:{ package features enabled by each package version}`. 
For example, if we run `cargo run --bin full get_deps rand 0.8.5`, the output should be
```Shell
raph: Graph {
  - cfg-if v1.0.0
  ## Our virtual package (dep). Its dependencies are direct dependencies while others are indirect dependencies.
  - dep v0.1.0 (/home/loancold/Projects/test_rust/docker_rust/Cargo-Ecosystem-Monitor/Code/rust_deps)
    - rand v0.8.5
  - getrandom v0.2.7
    - cfg-if v1.0.0
    - libc v0.2.129
    - wasi v0.11.0+wasi-snapshot-preview1
  - libc v0.2.129
  xxx Omitted
}

features: {
  syn v1.0.99: ["clone-impls", "default", "derive", "parsing", "printing", "proc-macro", "quote"]
  serde v1.0.143: ["default", "derive", "serde_derive", "std"]
  xxx Omitted
}
```

### Known Issue

The memory usage will continue rising when running the program, and the program usually not exits as some packages block the processing in the end. You can cancle the program and rerun it to make everything right until almost all packages in the ecosystem have been resolved.