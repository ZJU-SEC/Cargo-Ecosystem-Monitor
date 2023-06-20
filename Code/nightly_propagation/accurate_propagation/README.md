### Architecture

Before `cargo run`, you have to run projects "fetch_features" and "rust_deps" first to create related DB. This project will identify RUF impacts using RUF info and EDG raw data.

Based on DB table `tmp` which stores the relation between feature and nightly feature, we will build two extra DB tables.

- `feature_propagation_indir_relation`: Find all indir dependents of versions with nightly features.
- `feature_propagation_ver_status`: Show if the version has been resolved, which is for breaking-point rerun.
  