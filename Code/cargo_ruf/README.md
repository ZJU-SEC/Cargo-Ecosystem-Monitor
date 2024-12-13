FIXME: update this readme.

# Ruf Audit

This project aims at providing RUF audit tools to mitigate RUF impacts, including two crates:
- `ruf_audit`:Main mitigation tools. More details can be found at its readme file.
- `audit_pipeline` Evaluation of our tools. This pipeline will run `ruf_audit` in all crates with ruf issues, as detected before.


## Usage

Preparations ahead of running:

- database: we need all result data ready in postgreSQL, as described in the root readme file. (file alltables_20220811.sql). We use impacted package versions as our database. The impacted packages are determined by the `Total Impact` script in `Cargo-Ecosystem-Monitor/Code/scripts/research_results.sql`.
- crates source code: we need all crate sources with ruf issues at `Cargo-Ecosystem-Monitor/crates_donwloader/on_process`. To specify the crates we want to download, you can execute the script below to insert items we want to download. But when you have run our crate downloader, things may be more complicated.
    ```sql
    INSERT INTO download_status
    SELECT crate_id, id, name, num, 'undownloaded' FROM versions_with_name
    WHERE id in (
    SELECT DISTINCT(id) FROM tmp_ruf_impact WHERE status = 'removed' or status = 'unknown' 
    )
    ```
    After 'on_process' ready, remember to add a rust-toolchain file under the directory `on_process` in the `crate_downloader` project, specifying 'nightly-2022-05-19'.
- ruf_audit: please make sure the `ruf_audit` is ready to use through running the compilation command `cargo build` under the directory `ruf_audit`.

And now you can run the `audit_pipeline` simply with `cargo run` under its directory.

ATTENTION:

1. We have found that if your shell contains env `RUSTC`, our audit tools may fail but behave successfully. You should unset the env though command `unset RUSTC` before execute any tools in the Ruf Audit project.
   
2. As the audit process needs heavy CPU and memory usage. We recommend that you limit the CPU and memory usage when running the audit pipeline. In linux-based OS, you can use cgroup to manage.
   
```bash
sudo apt-get install cgroup-tools
sudo cgcreate -t $USER:$USER -a $USER:$USER  -g cpu,memory:/rufAuditGroup
cgset -r memory.limit_in_bytes=32G rufAuditGroup # You can set memory limit according to your machine
cgset -r cpu.cfs_quota_us=75000 rufAuditGroup 
cgset -r cpu.cfs_period_us=100000 rufAuditGroup # Allows at most 75000 us running every 100000us, or max CPU usage at 75%.
cgexec -g memory,cpu:rufAuditGroup $your_command & # $your_command here should be `cargo run` under `audit_pipeline` directory.
```