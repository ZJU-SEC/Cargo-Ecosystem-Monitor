# Cargo RUF

To mitigate RUF impacts, we also developed RUF Detector named cargo_ruf to detect given Rust projects to provide useful information on enabled RUF. Most importantly, it automatically tries to recover the package if it suffers from compilation failure introduced by enabled RUF. For compatibility concerns, it can also be integrated into Cargo, which is the Rust official package manager.

How to use?
```shell
# run in the root of target crate
$ cargo-ruf
```

If you have specific package feature configurations, you can run as `cargo`, for example:
```shell
$ cargo-ruf --features "default"
```

See [custom subcommands](https://doc.rust-lang.org/cargo/reference/external-tools.html#custom-subcommands) to integrate our tools into Cargo.

```Shell
cargo install --path .
cargo ruf
# or "cargo-ruf"
```

### Demo

Switching to `Code/demo/ruf_failure_demo` and building it, you will face a compilation error (using newest compiler). After running `cargo-ruf`, which takes a while, you will recovery from it and can successfully build it using `cargo build`.