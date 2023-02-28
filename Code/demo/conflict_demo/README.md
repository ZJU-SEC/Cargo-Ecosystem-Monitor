pack1 toml:

```
[package]
name = "pack1"
version = "0.1.0"
edition = "2021"

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[dependencies]
rand = "=0.6.2"
pack2 = { path = "../pack2" }
```



Pack2 toml:

```
[package]
name = "pack2"
version = "0.1.0"
edition = "2021"

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[dependencies]
rand = "0.6.*"
```





When generate lockfile on pack1, we have:

```
[[package]]
name = "rand"
version = "0.6.2"
```





When generate lockfile on pack2, we have:

```
[[package]]
name = "rand"
version = "0.6.5"
```





Run resolve on pack1 we have:

```
graph: Graph {
  - autocfg v0.1.8
    - autocfg v1.1.0
  - autocfg v1.1.0
  - bitflags v1.3.2
  - cloudabi v0.0.3
    - bitflags v1.3.2
  - fuchsia-cprng v0.1.1
  - libc v0.2.119
  - pack1 v0.1.0
    - pack2 v0.1.0
    - rand v0.6.2
  - pack2 v0.1.0
    - rand v0.6.2
```

