# Cargo-Ecosystem-Monitor

We focus on the research problem: Is there any security issues that have spread through dependencies across the ecosystem? We choose Rust/Cargo ecosystem as our target, as Rust highlights its security development along the way.

Actually we divide it into two parts:
- Propagation: **Package granularity**. Mainly focus on the Rust package manager. Typically, we won't analyse any codes inside the package. So the research results only present an overview of the whole ecosystem. Our project of this part is called "Cargo Ecosystem Monitor".
- Reachibility and Triggerbility Detection(Pending): In this case, we must statically(main form) or dynamically analyse the **codes inside crates**.
  - Function revocation: Is specific bug in one crate invoked by another crates through dependencies?
  - Codes clone: In some ways, we have to manually copy+paste others' codes as we want to modify them, which bypasses Cargo. This may cause delay update of related codes and they are often hard to maintain. Example: https://users.rust-lang.org/t/dependency-conflict/61807/5
  - Unsafe <-> Safe function: Can goalkeeper funtion make itself safe? It need propagation, type safety and other types of analysis.
  - Untrusted maintainer: These maintainers can insert vulnerabilities into their packages and then affect other pakcages through dependencies.

Now, we only focus on the "propagation" which is much easier and more fundamental. After we construct the ecosystem dependency graph, we can dive into the ecosystem to discover more ecosystem-scale impacts from different dimensions.



### Rust Unstable Feature Analysis

Our first target issue in our research is the **Rust unstable feature (RUF)**. We observe that the compiler allows developers to use RUF to extend the functionalities of the compiler. However, RUF may introduce vulnerabilities to Rust packages. Moreover, removed RUF will make packages using it suffers from compilation failure, thus breaking their usability and reliability. Even worse, the compilation failure propagates through package dependencies, causing potential threats to the entire ecosystem. Although RUF is widely used by Rust developers, unfortunately, to the best of our knowledge, its usage and impacts on the whole Rust ecosystem have not been studied so far.

To fill this gap, we conduct the first in-depth study to analyze RUF usage and impacts on the whole Rust ecosystem. More specifically, we first extract RUF definitions from the compiler and usage from packages. Then we resolve all package dependencies for the entire ecosystem to quantify the RUF impacts on the whole ecosystem.

By resolving the above challenges, we design and implement RUF extractor to extract all RUF definitions and configurations. 
We identify the semantics of RUF configuration defined by developers for precise RUF impact analysis.
To quantify RUF impacts over the whole Rust ecosystem, we define factors that affect impact propagation and generate a precise EDG for the entire Rust ecosystem (2022-08-11).

We analyzed all Rust compiler versions and obtained 1,875 RUF. We further analyzed all packages on official package database crates.io and resolve 592,183 package versions to get 139,525,225 direct and transitive dependencies and 182,026 RUF configurations. 

Our highlighted findings are: 1) About half of RUF (47\%) are not stabilized in the latest version of the Rust compiler;
2) Through dependency propagation, RUF can impact 259,540(44\%) package versions, causing at most 70,913 (12\%) versions suffer from compilation failure. 

These findings reveal the importance of stabilizing RUF for Rust ecosystem security and reliability.

### How to build our project

There are sub-projects under our projects. Also, they are loosely or closely connected to support our research. Before running them, we have to build external environment as follows. Some of them can be done using scripts, but the others need manual work. You can use our `Makefile` for help, but you can't imagine that it will complete all the tasks.

1. Tool: Rust, PostgreSQL. We need you to create PostgreSQL with account 'postgres' and password 'postgres', we'll use this accout and DB for further data analysis. While installing, you may face trouble. Refer following websites for help:
   1. https://stackoverflow.com/questions/55038942/fatal-password-authentication-failed-for-user-postgres-postgresql-11-with-pg
   2. https://forge.rust-lang.org/infra/other-installation-methods.html
2. Import data and compile code. 
   1. Get to website [crates.io](https://crates.io/data-access) and follows Step 2 of The `README.md` in the gz package, which tells you how to set up your DB. You should setup your database called 'crates'.
   2. The directory `crates.io-index` points to the index of crates.io. The index of crates should be the same with our database. As a result, we need to change checkout the git commits to meet the time of our database. For example, we download crates.db named `2023-01-11-020041`, we need to checkout the commits to or near 2023-01-11 02:00:41.
   3. Import modifed Rustc codes under directory `rust`, and build Rustc through the guide in the README file.
3. Now you've setup your data and tools. You may need further environment setup under specific projects, which is also detailedly described in directory `code`. Refer to it for further analysis.




### Other related projects

- RustSec: https://github.com/rustsec/rustsec
- Cargo-lock: https://github.com/RustSec/rustsec/tree/main/cargo-lock
- Cargo-audit: https://github.com/RustSec/rustsec/tree/main/cargo-audit
- Others: See [OtherTools](./OtherTools.md)
