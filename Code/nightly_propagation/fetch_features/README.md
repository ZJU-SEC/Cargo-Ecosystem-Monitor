# RUF Configuration Fetch

In this project, we will fetch RUF configuration defined by crates in the Rust ecosystem. The results are stored in the DB table `version_feature`.

We spawn 20 threads for processing by default.

### Preliminaries

This project needs pre-downloaded crates source code in path `CRATESDIR` (configured in `main.rs`), and the modified Rust compiler in path `RUSTC` (configured in `util.rs`). Make sure they are prepared.

To pre-download source codes of all crates, you can use project `crate_downloader` in the parent directory. The modified Rust compiler is in `Cargo-Ecosystem-Monitor/rust`. Follow the readme file there to build the compiler.

The `CRATESDIR` and `RUSTC` values are set according to our dockerfile structure by default. If you are replicating our research results using our dockerfile, you do not need to change them. But you have to run preliminary projects.

### Usage

Online usage will download crates online while processing. In offline mode, we will use the downloaded crates source code stored in path `CRATESDIR`.

Offline mode is enabled by default. You do not need to change it in common cases.


Run `cargo run` in your shell to start.

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