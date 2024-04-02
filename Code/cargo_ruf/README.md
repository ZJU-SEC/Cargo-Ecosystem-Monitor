# Ruf Audit

This part it the mitigation tools and its evaluations, including two crates:
- `ruf_audit` is the mitigation tools, more details can be found at its readme file.
- `audit_pipeline` is the pipeline we used to evaluation our tools. This pipeline will run `ruf_audit` in all crates with ruf issues, as detected before.


## Usage

Before the evaluation, we have to do some preparings:
- database: we need all result data ready in postgres (file alltables_20220811.sql).
- crates: we need all crate sources with ruf issues at 'crates_donwloader/on_process'. To get all impacted crates, you should first build `tmp_ruf_impact` table,  and then select all crates using non-compilable rufs. The `donwload_status` table can be updated as:
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