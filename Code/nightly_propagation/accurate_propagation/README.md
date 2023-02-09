### Architecture

Based on DB table `tmp` which stores the relation between feature and nightly feature, we will build two extra DB tables.

- `feature_propagation_indir_relation`: Find all indir dependents of versions with nightly features.
- `feature_propagation_ver_status`: Show if the version has been resolved, which is for breaking-point rerun.
  