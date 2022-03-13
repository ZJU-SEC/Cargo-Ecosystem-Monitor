**Set up local registry**

1. Clone remote registry

```bash
git clone https://github.com/rust-lang/crates.io-index
```

2. Change .cargo/config.toml

```
[source]

[source.mirror]
registry = "file:///absolute/path/to/crates.io-index/dir"

[source.crates-io]
replace-with = "mirror"
```



**Local Speedup**

resolve `http 0.2.6`

```
local - 0.32s in total.
remote - 17.42s for the first run and 0.32s for the second.
```



resolve `cargo 0.60.0`

```
local - 1.21s in totoal.
remote - 18.92s for the first run and 1.20s for the second.
```



It seems that set a local registry won’t imporve too much except the first time run. Besides, even when using a local registry, cargo just take it as a local git repo and sync it into `~/.cargo/registry` (That’s quite stupid).
