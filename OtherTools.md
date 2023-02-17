### Discovery

Cargo.io has lots of Rust packages information, and we have lots of related projects to get related information.

Now, maybe we can build database according to crates.io, and use tools to analyse every Rust project, whether it's about security supply chain or CVE dependency graph.

Problem:

- Edition2021 is required: Features in 2021 is unstable, we need to update the toolchain.[substrate - Unable to specify `edition2021` in order to use unstable packages in Rust - Stack Overflow](https://stackoverflow.com/questions/69848319/unable-to-specify-edition2021-in-order-to-use-unstable-packages-in-rust)

Now I list some useful projects and their outputs.

### Cargo-Supply-Chain:

Results are partially as follows.



##### Command: `cargo supply-chain publishers`

> The following individuals can publish updates for your dependencies:
>
>  1. alexcrichton via crates: bitflags, bumpalo, cc, cfg-if, env_logger, filetime, flate2, futures, futures-io, jobserver, js-sys, libc, libm, log, miow, openssl-probe, pkg-config, rand, rand_core, regex, regex-syntax, scoped-tls, socket2, toml, unicode-normalization, unicode-segmentation, unicode-width, unicode-xid, uuid, wasi, wasm-bindgen, wasm-bindgen-backend, wasm-bindgen-futures, wasm-bindgen-macro, wasm-bindgen-macro-support, wasm-bindgen-shared, web-sys
>  2. kdy1 via crates: ast_node, enum_kind, from_variant, is-macro, pmutil, string_enum, swc_atoms, swc_bundler, swc_common, swc_ecma_ast, swc_ecma_codegen, swc_ecma_codegen_macros, swc_ecma_dep_graph, swc_ecma_loader, swc_ecma_parser, swc_ecma_transforms, swc_ecma_transforms_base, swc_ecma_transforms_classes, swc_ecma_transforms_macros, swc_ecma_transforms_optimization, swc_ecma_transforms_proposal, swc_ecma_transforms_react, swc_ecma_transforms_typescript, swc_ecma_utils, swc_ecma_visit, swc_ecmascript, swc_eq_ignore_macros, swc_fast_graph, swc_graph_analyzer, swc_macros_common, swc_visit, swc_visit_macros
>
> 
>
> All members of the following teams can publish updates for your dependencies:
>
>  1. "github:servo:cargo-publish" (https://github.com/servo) via crates: brotli, core-foundation, core-foundation-sys, data-url, fnv, form_urlencoded, idna, new_debug_unreachable, percent-encoding, smallvec, string_cache, string_cache_codegen, unicode-bidi, url, utf-8
>  2. "github:rustwasm:core" (https://github.com/rustwasm) via crates: js-sys, wasm-bindgen, wasm-bindgen-backend, wasm-bindgen-futures, wasm-bindgen-macro, wasm-bindgen-macro-support, wasm-bindgen-shared, web-sys

This shows members that can influence this project by dependencies as individuals or team members.

  

##### Command: `cargo supply-chain publishers`

> Dependency crates with the people and teams that can publish them to crates.io:
>
> 1. deno_graph: team "github:denoland:engineering", AaronO, bartlomieju, crowlKats, dsherret, kitsonk, kt3k, lucacasonato, piscisaureus, ry
> 2. string_cache: team "github:servo:cargo-publish", Manishearth, SimonSapin, asajeffrey, bholley, jdm, kmcallister, larsbergstrom, metajack, pcwalton
> 3. deno_ast: team "github:denoland:engineering", AaronO, bartlomieju, dsherret, kitsonk, kt3k, lucacasonato, piscisaureus, ry
> 4. import_map: team "github:denoland:engineering", AaronO, bartlomieju, dsherret, kitsonk, kt3k, lucacasonato, piscisaureus, ry

It shows each crate with corresponding contributors and teams.

### Cargo-geiger

Able to use, but faces shutdown during execution in some situation.

> Metric output format: x/y
>     x = unsafe code used by the build
>     y = total unsafe code found in the crate
>
> Symbols: 
>     :) = No `unsafe` usage found, declares #![forbid(unsafe_code)]
>     ?  = No `unsafe` usage found, missing #![forbid(unsafe_code)]
>     !  = `unsafe` usage found
>
> Functions  Expressions  Impls  Traits  Methods  Dependency
>
> 0/0        0/0          0/0    0/0     0/0      :) cargo-geiger 0.11.2
> 15/18      442/449      3/3    0/0     11/11    !  ├── anyhow 1.0.52
> 0/26       0/623        0/6    0/0     0/5      ?  │   └── backtrace 0.3.56
> 0/0        0/23         0/0    0/0     0/0      ?  │       ├── addr2line 0.14.1
> 0/0        0/51         0/2    0/0     0/0      ?  │       │   ├── gimli 0.23.0
> 0/0        37/42        1/1    0/0     0/0      !  │       │   │   └── indexmap 1.6.2
> 2/2        1006/1098    16/19  0/0     35/39    !  │       │   │       ├── hashbrown 0.9.1
> 0/0        4/4          0/0    0/0     0/0      !  │       │   │       │   └── serde 1.0.132
> 0/0        0/0          0/0    0/0     0/0      ?  │       │   │       │       └── serde_derive 1.0.132
> 0/0        12/12        0/0    0/0     3/3      !  │       │   │       │           ├── proc-macro2 1.0.36
> 0/0        0/0          0/0    0/0     0/0      :) │       │   │       │           │   └── unicode-xid 0.2.1
> 0/0        0/0          0/0    0/0     0/0      ?  │       │   │       │           ├── quote 1.0.9
> 0/0        12/12        0/0    0/0     3/3      !  │       │   │       │           │   └── proc-macro2 1.0.36
> 0/0        47/47        3/3    0/0     2/2      !  │       │   │       │           └── syn 1.0.85
> 0/0        12/12        0/0    0/0     3/3      !  │       │   │       │               ├── proc-macro2 1.0.36
> 0/0        0/0          0/0    0/0     0/0      ?  │       │   │       │               ├── quote 1.0.9
> 0/0        0/0          0/0    0/0     0/0      :) │       │   │       │               └── unicode-xid 0.2.1
> 0/0        4/4          0/0    0/0     0/0      !  │       │   │       └── serde 1.0.132
> 0/0        0/0          0/0    0/0     0/0      ?  │       │   ├── rustc-demangle 0.1.18
> 1/1        392/392      7/7    1/1     13/13    !  │       │   └── smallvec 1.6.1
> 0/0        4/4          0/0    0/0     0/0      !  │       │       └── serde 1.0.132
> 0/0        0/0          0/0    0/0     0/0      ?  │       ├── cfg-if 1.0.0
> 0/20       12/327       0/2    0/0     2/30     !  │       ├── libc 0.2.112
> 0/0        0/0          0/0    0/0     0/0      :) │       ├── miniz_oxide 0.4.4
> 0/0        0/0          0/0    0/0     0/0      :) │       │   └── adler 1.0.2
> 0/0        0/21         0/0    0/1     0/0      ?  │       ├── object 0.23.0

It only analyses the unsafe usage by supply chain rather than crates. It gives an overview of package unsafe pollution.

### Cargo-metadata

Output examples:

> {"packages":[{"name":"addr2line","version":"0.14.1","id":"addr2line 0.14.1 (registry+https://github.com/rust-lang/crates.io-index)","license":"Apache-2.0/MIT","license_file":null,"description":"A cross-platform symbolication library written in Rust, using `gimli`","source":"registry+https://github.com/rust-lang/crates.io-index","dependencies":[{"name":"rustc-std-workspace-alloc","source":"registry+https://github.com/rust-lang/crates.io-index","req":"^1.0.0","kind":null,"rename":"alloc","optional":true,"uses_default_features":true,"features":[],"target":null,"registry":null},{"name":"compiler_builtins","source":"registry+https://github.com/rust-lang/crates.io-index","req":"^0.1.2","kind":null,"rename":null,"optional":true,"uses_default_features":true,"features":[],"target":null,"registry":null},{"name":"rustc-std-workspace-core","source":"registry+https://github.com/rust-lang/crates.io-index","req":"^1.0.0","kind":null,"rename":"core","optional":true,"uses_default_features":true,"features":[],"target":null,"registry":null},{"name":"cpp_demangle","source":"registry+https://github.com/rust-lang/crates.io-index","req":"^0.3","kind":null,"rename":null,"optional":true,"uses_default_features":false,"features":[],"target":null,"registry":null},{"name":"fallible-iterator","source":"registry+https://github.com/rust-lang/crates.io-index","req":"^0.2","kind":null,"rename":null,"optional":true,"uses_default_features":false,"features":[],"target":null,"registry":null},{"name":"gimli","source":"registry+https://github.com/rust-lang/crates.io-index","req":"^0.23","kind":null,"rename":null,"optional":false,"uses_default_features":false,"features":["read"],"target":null,"registry":null},{"name":"object","source":"registry+https://github.com/rust-lang/crates.io-index","req":"^0.22","kind":null,"rename":null,"optional":true,"uses_default_features":false,"features":["read"],"target":null,"registry":null},{"name":"rustc-demangle","source":"registry+https://github.com/rust-lang/crates.io-index","req":"^0.1","kind":null,"rename":null,"optional":true,"uses_default_features

The complete output follows json style, and is quite complicated. I use python scripts to split each keys, and get top keys :

> packages
> workspace_members
> resolve
> target_directory
> version
> workspace_root
> metadata


### Execute our projects in server using SSH

To avoid terminating the execution after SSH termination, you should use `nohup` by using shell command `nohup cargo run >&/dev/null&`. The `/dev/null` is used to avoid catching output, because it occupies large number of storage.