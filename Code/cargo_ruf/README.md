FIXME: update this readme.

# Ruf Audit

This project aims at providing RUF audit tools to mitigate RUF impacts, including two crates:
- `ruf_audit_virtual`: Main mitigation tools. More details can be found at its readme file.
- `virt_audit_pipeline`: Evaluation of our tools. This pipeline will run `ruf_audit_virtual` in all crates with ruf issues, as detected before.


## Usage

Preparations ahead of running:
- Database: We need all result data ready in postgreSQL, as described in the root readme file. (file alltables_20220811.sql).
    - ruf_audit_virtual: This tool need some extra tables, you can create them as follows:
        ```sql
        CREATE VIEW dependencies_with_name AS
        SELECT dependencies.*, crates.name AS crate_name
        FROM dependencies
        JOIN crates
        ON dependencies.crate_id = crates.id

        CREATE TABLE version_ruf AS
        SELECT versions_with_name.id, versions_with_name.name, versions_with_name.num, versions_with_name.crate_id, version_feature.conds, version_feature.feature
        FROM versions_with_name
        JOIN version_feature
        ON versions_with_name.id = version_feature.id

        UPDATE version_ruf SET conds = NULL WHERE conds = ''
        ```
    - virt_audit_pipeline: This pipeline will run `ruf_audit_virtual` in all crates with ruf issues, as detected before. A process table shall be created before running the pipeline, here is an example:
        ```sql
        CREATE TABLE virt_audit_process AS (
        SELECT DISTINCT(ver) as id, 'undone' as status
        FROM tmp_ruf_impact
        WHERE status = 'removed' OR status = 'unknown')
        ```

        Besides the process table, one more directory are needed as workspace dir. You can create it as follows:
        ```bash
        mkdir -p virt_audit_jobs/.cargo
        touch virt_audit_jobs/.cargo/config.toml
        tree
        virt_audit_pipeline
        └── virt_audit_jobs
            └── .cargo
                └── config.toml
        ```
        **Please DONNOT forget to set the `config.toml` file to our crates.io.**

And now you can run the `virt_audit_pipeline` simply with `cargo run` under its directory.

ATTENTION:

1. As the audit process needs heavy CPU and memory usage. We recommend that you limit the CPU and memory usage when running the audit pipeline. In linux-based OS, you can use cgroup to manage.
    ```bash
    sudo apt-get install cgroup-tools
    sudo cgcreate -t $USER:$USER -a $USER:$USER  -g cpu,memory:/rufAuditGroup
    cgset -r memory.limit_in_bytes=32G rufAuditGroup # You can set memory limit according to your machine
    cgset -r cpu.cfs_quota_us=75000 rufAuditGroup 
    cgset -r cpu.cfs_period_us=100000 rufAuditGroup # Allows at most 75000 us running every 100000us, or max CPU usage at 75%.
    cgexec -g memory,cpu:rufAuditGroup $your_command & # $your_command here should be `cargo run` under `audit_pipeline` directory.
    ```