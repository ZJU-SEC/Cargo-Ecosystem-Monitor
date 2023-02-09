# Code

This directory contains multiple sub-projects for various analysis of the Rust ecosystem.

- `accuracy_evaluation`: Evaluate the accuracy of our dependency resolution tool. Should be done after project `rust_deps`.
- `advisory_scanner`: Scan the advisory impact range across the Rust ecosystem according to provided advisory data in json file. Should be done after project `rust_deps`.
- `conflict_demo`: This demo describes the dependency entanglement situation (temporarily defined by us), which means dependencies can affect each other, causing different final decisions on versions (or even package features).
- `crate_downloader`: It is used to download source codes of all Rust packages.
- `demo`: For private test only. Should not be used.
- `nightly_propagation`: RUF analysis tools.
  - `accurate_propagation`: Accurately evaluate the impacts of RUF. Should be done after project `rust_deps` and `fetch_features`.
  - `fetch_features`: Fetch RUF configuration defined by crates in the Rust ecosystem. Should be done after project `crate_downloader`.
  - `run_propagation`: Deprecated. Should not be used.
  - `test_feature`: RUF lifetime analysis. This project scans the Rustc source code and extract RUF definition information.
- `rust_deps` (First): Rust Ecosystem Dependency Graph (EDG) Generator. It is recommended that you first read this project to build the EDG.
- `scripts`: SQL scripts to analyze the Crates database.