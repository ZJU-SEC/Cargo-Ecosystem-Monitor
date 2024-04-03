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
    After 'on_process' ready, remember to add a rust-toolchain file, specifying 'nightly-2022-05-19'.
- cargo_usage: for multithreading, please `mkdir` before runing:
    ```sh
        cargo_usage
        ├── home0
        ├── home1
        ├── home2
        └── home3
    ```
- ruf_audit: please make sure the `ruf_audit` is ready to use, and the `ruf_audit` is in the same directory with `audit_pipeline`.

And now you can run the pipeline simply with `cargo run`.