# Cargo RUF
A tools to detect ruf usage and fix them if possible.

How to use?
```shell
# run in the root of target crate
$ cargo_ruf
```

If you have specific package feature configurations, you can run as `cargo`, for example:
```shell
$ cargo_ruf --features "default"
```