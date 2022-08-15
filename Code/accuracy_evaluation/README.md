# Accuracy Evaluation

This crate is used to evaluate the accuracy of the Resolution Pipeline of "Cargo Ecosystem Monitor". Before executing any programs in this project, you should build your crates postgresql database from Crates.io first.

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