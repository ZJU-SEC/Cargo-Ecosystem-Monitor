# Code

This directory contains multiple sub-projects for various analysis of the Rust ecosystem. All work should be done after the env build (Step 0) following the root directory guide.


![Tool Structure](Arch.png)

The figure shows main tool structure of this project. 

- `accuracy_evaluation`: Evaluate the accuracy of our dependency resolution tool. Should be done after project `rust_deps`.
- `advisory_scanner`: Scan the advisory impact range across the Rust ecosystem according to provided advisory data in json file. Should be done after project `rust_deps`. This tool is not included in RUF study, but is extended for vulnerability study in the Rust ecosystem, and can reuse the existing architecture to achieve the goal.
- `cargo_ruf`: RUF detector of Rust projects. It now tries to recover packages that suffer from compilation failure due to RUF impacts. It can be integrated into Cargo.
- `crate_downloader`: It is used to download source codes of all Rust packages.
- `demo`: For private test only. Should not be used.
- `nightly_propagation`: RUF analysis tools.
  - `accurate_propagation`: Accurately evaluate the impacts of RUF. Should be done after project `rust_deps` and `fetch_features`.
  - `fetch_features`: Fetch RUF configuration defined by crates in the Rust ecosystem. Should be done after project `crate_downloader`.
  - `ruf_mitigation_analysis`: It scans the whole Rust ecosystem to analyze how many packages can recover from RUF threats, like compilation failure. It is a validation tool to estimate the success rate of `cargo_ruf`.
  - `run_propagation`: Deprecated. Should not be used.
  - `test_feature`: RUF lifetime analysis. This project scans the Rustc source code and extracts RUF definition information. It can also further analyze the abnormal RUF lifetime and virtualize it.
- `rust_deps` (First): Rust Ecosystem Dependency Graph (EDG) Generator. It is recommended that you first read this project to build the EDG.
- `scripts`: SQL scripts to prebuild databases and analyze the Crates database. After building all necessary databases, you can use SQL scripts here to generate "research results" here.


Execution flow dependency:

1.1 rust_deps         -> 1.2 accuracy_evaluation (Ensure correctness)   -> 1.3 accurate_propagation

2.1 crate_downloader  -> 2.2 fetch_features -> 1.3

3.1 test_feature      -> 1.3


### Code Count (2024-03-08)

Summary:
Projects: 11
Code count (Pure): 12846 LoC

```

accuracy_evaluation
-------------------------------------------------------------------------------
Language                     files          blank        comment           code
-------------------------------------------------------------------------------
Rust                             9            192            202           1417
-------------------------------------------------------------------------------
SUM:                             9            192            202           1417
-------------------------------------------------------------------------------

advisory_scanner
-------------------------------------------------------------------------------
Language                     files          blank        comment           code
-------------------------------------------------------------------------------
Rust                             1             20             44            122
-------------------------------------------------------------------------------


cargo_ruf
-------------------------------------------------------------------------------
Language                     files          blank        comment           code
-------------------------------------------------------------------------------
Rust                            22            332            262           3280
TOML                             8             18              7             72
Python                           1              5              6             41
Markdown                         1              5              0             22
-------------------------------------------------------------------------------
SUM:                            32            360            275           3415
-------------------------------------------------------------------------------

crate_downloader
-------------------------------------------------------------------------------
Language                     files          blank        comment           code
-------------------------------------------------------------------------------
Rust                             3             46             12            259
-------------------------------------------------------------------------------
SUM:                             3             46             12            259
-------------------------------------------------------------------------------

github_ecosystem
-------------------------------------------------------------------------------
Language                     files          blank        comment           code
-------------------------------------------------------------------------------
Python                           1             48            131            276
make                             1              3              1              4
-------------------------------------------------------------------------------
SUM:                             2             51            132            280
-------------------------------------------------------------------------------

nightly_propagation/accurate_propagation
-------------------------------------------------------------------------------
Language                     files          blank        comment           code
-------------------------------------------------------------------------------
Rust                             1             39             29            332
-------------------------------------------------------------------------------

nightly_propagation/fetch_features
-------------------------------------------------------------------------------
Language                     files          blank        comment           code
-------------------------------------------------------------------------------
Rust                             3            112             40            739
-------------------------------------------------------------------------------
SUM:                             3            112             40            739
-------------------------------------------------------------------------------


nightly_propagation/ruf_mitigation_analysis
-------------------------------------------------------------------------------
Language                     files          blank        comment           code
-------------------------------------------------------------------------------
Rust                             3             20             26           2150
-------------------------------------------------------------------------------
SUM:                             3             20             26           2150
-------------------------------------------------------------------------------

nightly_propagation/test_feature
-------------------------------------------------------------------------------
Language                     files          blank        comment           code
-------------------------------------------------------------------------------
Rust                             2             81             90            415
Python                           2             26             11             98
make                             1              0              0              3
-------------------------------------------------------------------------------
SUM:                             5            107            101            516
-------------------------------------------------------------------------------


rust_deps
-------------------------------------------------------------------------------
Language                     files          blank        comment           code
-------------------------------------------------------------------------------
Rust                             7            311            815           2234
-------------------------------------------------------------------------------
SUM:                             7            311            815           2234
-------------------------------------------------------------------------------

scripts
-------------------------------------------------------------------------------
Language                     files          blank        comment           code
-------------------------------------------------------------------------------
Python                           3            101            251            722
SQL                              9            127            176            660
-------------------------------------------------------------------------------
SUM:                            12            228            427           1382
-------------------------------------------------------------------------------

```
