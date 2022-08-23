# Accuracy Evaluation

This crate is used to evaluate the accuracy of the Resolution Pipeline of "Cargo Ecosystem Monitor". Before executing any programs in this project, you should:

1. build your crates postgresql database from Crates.io first. dbname=crates user=postgres password=postgres.
2. Run project `rust_deps` to build table `dep_version`.
3. In step 2, you need to set .cargo environment to specify certain cargo index cache. In this project, you also need to do so, so that standard benchmark uses the same index.
4. Run scripts `prebuild.sql` to build neccesary tables. 
5. Run `cargo run --bin benchmark_dataset` under this project. This will automatically generate dataset under directory `output`.
6. Run `cargo run --bin pipeline_evaluation` under this project. This will automatically generate pipeline resolution results under directory `output`, and also store comparison results.
7. Run `cargo run --bin results_summary` under this project. This will automatically analyze comparison results and print them in command line.

When re-run the program, you have to manually clear all the cache data:
- Delete directory `output` to avoid reconsidering duplicate crates in results summary.
- Delete DB table `accuracy_evaluation_status` to clear all current status.
- If you want to continue the stopped process, just re-run. Nothing needs to be done.
- If you want to change the dataset stratety, you should change the code. Also, do as re-run.

The evaluation machanism works as follows:

1. Find top 1000(customized) crates with most direct dependencies.
2. Download crate source code from official database.
3. Use Cargo to resolve the dependencies of each crate in local and real environment.
4. Compare the resolution results with database created by our Resolution Pipeline.

Some differences can be tolerated, like new crates are published between resolution and evaluation.

### Architecture

We have three binary programs, they should be executed in order:

1. benchmark_dataset: It downloads top crates, resolve them by using `cargo tree` and store results in local.
2. pipeline_evaluation: This process should be executed after building database `dep_version` using project `rust_deps`. It compares cargo tree dependency results with the ones resolved by our dependency resolution pipeline, which is `rust_deps`. The pipeline resolution and comparison results are stored in local. 
3. results_summary: This process will summarize all comparison results and give final judgement.


### Broken-point Continuingly-transferring

To maintain the resolving process, we build a database table `accuracy_evaluation_status`. In `status` field, there are three possible values, which are `unevaluated`, `resolved`, `evaluated`, `fails`.

- `unevaluated`: Not touched.
- `resolved`: Resolved by cargo tree.
- `evaluated`: Resolved by both cargo tree and pipeline. Work done.
- `fails`: Crate resolution process fails, and won't be continued. Mostly caused from downloader. 


### Inaccuracy Types

Our pipeline resolution results may differ from standard results. The main reasons are:

- Dependency entanglement: Due to Cargo dependency cache mechanism, used dependencies are influenced by unused dependencies at certain time. The cache mechanism will merge crates if different dependencies requirements can be satisfied. But that introduces uncertainly transparent to the crate developer as the dependency changed from what they think it should be.
  - Example: Crate `p2pands-rs-v0.4.0` depends on `openmls-v0.4.1`, which optionally depends on `rstest-v0.13.0`. At the same time, `p2pands-rs-v0.4.0` has a development dependency on `openmls-v0.4.1` with more features on. However, the crate `openmls-v0.4.1`  with more features on will be selected, as they are merged to this one. As a consequence, `rstest-v0.13.0` is dependent on. This change the behavior of what developers expected, as more features are opened by default. The code behavior might changed.-