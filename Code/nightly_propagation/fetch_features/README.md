# RUF Configuration Fetch

In this project, we will fetch RUF configuration defined by crates in the Rust ecosystem. The results are stored in the DB table `version_feature`.

We spawn 20 threads for processing by default.

### Preliminaries

This project needs pre-downloaded crates source code in path `CRATESDIR`, and modified Rust compiler in path `RUSTC`. Make sure they are prepared.

To pre-download source codes of all crates, you can use project "crate_downloader" in the parent directory. Modified Rust compiler is in root directory of Cargo Ecosystem Monitor.

### Usage

Online usage will download crates online while processing. In offline mode, we will use downloaded crates source code stored in path `CRATESDIR`.

Offline mode is enabled by default. You should change `CRATESDIR` to your own path. 

#### Online
To run online, please config `main.rs`:
```rust
run(workers: usize, todo_status: &str)
```
here:
- workers: number of threads
- todo_status: status to be processed ("undone", "fail")

#### Offline
To run offline, please config `main.rs`:
```rust
run_offline(workers: usize, todo_status: &str, home: &str)
```
here:
- workers: number of threads
- todo_status: status to be processed ("undone", "fail")
- home: where source files are stored