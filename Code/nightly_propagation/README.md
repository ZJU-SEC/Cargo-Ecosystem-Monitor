# RUF Analysis

Directory Structure:

- `accurate_propagation`: Accurately evaluate the impacts of RUF. Should be done after project `rust_deps` and `fetch_features`.
- `fetch_features`: Fetch RUF configuration defined by crates in the Rust ecosystem. Should be done after project `crate_downloader`.
- `ruf_mitigation_analysis`: It scans the whole Rust ecosystem to analyze how many packges can recover from RUF threats, like compilation failure. It is a validation tool to estimate the success rate of `cargo_ruf`.
- `run_propagation`: Deprecated. Should not be used.
- `test_feature`: RUF lifetime analysis. This project scans the Rustc source code and extract RUF definition information. It can also further analyze the abormal RUF lifetime and virtualise it.