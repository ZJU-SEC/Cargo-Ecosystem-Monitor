**Set up local registry**

1. Clone remote registry

```bash
git clone https://github.com/rust-lang/crates.io-index
```

2. Change .cargo/config.toml

```
[net]
git-fetch-with-cli = true

[source.mirror]
registry = "file:///absolute/path/to/crates.io-index/dir"

[source.crates-io]
replace-with = "mirror"
```
