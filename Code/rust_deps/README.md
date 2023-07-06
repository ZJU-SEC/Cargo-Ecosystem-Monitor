**Set up local registry**

0. Build your Crates.io DB (As told in root directory Readme file)

Get to [crates.io website](https://crates.io/data-access) and follows Step 2. The README.md in it will tell you how to set up your DB.

1. Clone remote registry (As told in root directory Readme file)

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

If you have run this tool, there will be extra tables in DB which are old. You should drop all these tables.


5. Run `cargo run`!

Be sure that you have corretly set up your database according to our guide in the project root directory. Moreover, if you can't connect to database, there might be sth wrong with the connection, including port(sometimes 5432, and sometimes 5434) and else.

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
wasi,0.11.0+wasi-snapshot-preview1,4
rand_core,0.6.3,2
wasi,0.11.0+wasi-snapshot-preview1,3
syn,1.0.99,5
serde_derive,1.0.143,4
getrandom,0.2.7,3
serde_derive,1.0.143,3
proc-macro2,1.0.43,5
quote,1.0.21,5
serde,1.0.143,2
libc,0.2.129,3
quote,1.0.21,3
getrandom,0.2.7,2
proc-macro2,1.0.43,7
libc,0.2.129,1
syn,1.0.99,4
serde,1.0.143,3
quote,1.0.21,4
unicode-ident,1.0.3,6
cfg-if,1.0.0,3
proc-macro2,1.0.43,6
rand_core,0.6.3,1
libm,0.1.4,2
serde_derive,1.0.143,2
libc,0.2.129,4
proc-macro2,1.0.43,4
unicode-ident,1.0.3,5
quote,1.0.21,6
unicode-ident,1.0.3,8
packed_simd_2,0.3.8,1
ppv-lite86,0.2.16,2
log,0.4.17,1
cfg-if,1.0.0,4
serde,1.0.143,1
```


### Known Issue

The memory usage will continue rising when running the program, and the program usually not exits as some packages block the processing in the end. You can cancle the program and rerun it to make everything right until almost all packages in the ecosystem have been resolved.