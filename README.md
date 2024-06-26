# Cargo-Ecosystem-Monitor

Rust ecosystem analysis, mainly the Cargo ecosystem.

Our first target issue is the **Rust unstable feature (RUF)**, published in [ICSE'24](https://dl.acm.org/doi/10.1145/3597503.3623352). See  [released full paper here](./RustUnstableFeature_ICSE24.pdf).

**WIP**: Though our repo follows a decoupled design, the code directory becomes more and more complex thus making reuse a big problem. We are now trying to separate each project into a separate repo for simplicity. See https://github.com/Cargo-Ecosystem-Monitor for the latest updates.

## Motivation

We focus on the research problem: Are there any security issues that have spread through dependencies across the ecosystem? We choose Rust/Cargo ecosystem as our target, as Rust highlights its security development along the way.

Actually, we divide it into two parts:
- Propagation: **Package granularity**. Mainly focus on the Rust package manager. Typically, we won't analyze any codes inside the package. So the research results only present an overview of the whole ecosystem. Our project for this part is called "Cargo Ecosystem Monitor".
- Reachability and Triggerbility Detection(Pending): In this case, we must statically(main form) or dynamically analyze the **codes** inside crates**.
  - Function revocation: Are there any specific bugs in one crate invoked by other crates through dependencies?
  - Codes clone: In some ways, we have to manually copy+paste others' codes as we want to modify them, which bypasses Cargo. This may cause delay update of related codes and they are often hard to maintain. Example: https://users.rust-lang.org/t/dependency-conflict/61807/5
  - Unsafe <-> Safe function: Can the goalkeeper function make itself safe? It needs propagation, type safety and other types of analysis.
  - Untrusted maintainer: These maintainers can insert vulnerabilities into their packages and then affect other packages through dependencies.

Now, we only focus on the "propagation" which is much easier and more fundamental. After we construct the ecosystem dependency graph, we can dive into the ecosystem to discover more ecosystem-scale impacts from different dimensions.



### Rust Unstable Feature Analysis (Published in ICSE'24)

Our first target issue in our research is the **Rust unstable feature (RUF)**. We observe that the compiler allows developers to use RUF to extend the functionalities of the compiler. However, RUF may introduce vulnerabilities to Rust packages. Moreover, removed RUF will make packages using it suffer from compilation failure, thus breaking their usability and reliability. Even worse, the compilation failure propagates through package dependencies, causing potential threats to the entire ecosystem. Although RUF is widely used by Rust developers, unfortunately, to the best of our knowledge, its usage and impacts on the whole Rust ecosystem have not been studied so far.

To fill this gap, we conduct the first in-depth study to analyze RUF usage and impacts on the whole Rust ecosystem. More specifically, we first extract RUF definitions from the compiler and usage from packages. Then we resolve all package dependencies for the entire ecosystem to quantify the RUF impacts on the whole ecosystem.

By resolving the above challenges, we design and implement RUF extractor to extract all RUF definitions and configurations. 
We identify the semantics of RUF configuration defined by developers for precise RUF impact analysis.
To quantify RUF impacts over the whole Rust ecosystem, we define factors that affect impact propagation and generate a precise EDG for the entire Rust ecosystem (2022-08-11).

We analyzed all Rust compiler versions and obtained 1,875 RUF. We further analyzed all packages on the official package database crates.io and resolve d592,183 package versions to get 139,525,225 direct and transitive dependencies and 182,026 RUF configurations. 

Our highlighted findings are: 1) About half of RUF (47\%) are not stabilized in the latest version of the Rust compiler;
2) Through dependency propagation, RUF can impact 259,540(44\%) package versions, causing at most 70,913 (12\%) versions suffer from compilation failure. To mitigate wide RUF impacts, we further design and implement the RUF compilation failure recovery tool that can recover up to 90% of the failure. We believe our techniques, findings, and tools can help to stabilize the Rust compiler, ultimately enhancing the security and reliability of the Rust ecosystem.

## How to build our project

There are sub-projects under our projects, which are loosely or closely connected to support our research. We intentionally do not merge them into a unified project, as we want to support extensive applications and diverse debug requirements. This may make the build process and documentation a little frustrating. We're trying to provide more build scripts and better documentation to make everything clearer. 

### Setup Environment

You can use our dockerfile which prepares all the environment for you. You can also build this project in your local machine, and customize the usage. Currently, our scripts and documentation are not detailed enough to support local build. We're making efforts. 


#### Dockerfile Build (Recommend)

Refer to [Reproduce Our Results](#reproduce-our-results) section for more details.


#### Local Build (Developing)

Before running them, we have to build an external environment as follows. Some of them can be done using scripts, but the others need manual work. You can use our `Makefile` for help, but you can't imagine that it will complete all the tasks. At the same time, we also provide dockerfile and docker image to build neccessary runtime environment and dependencies to build our projects. You can either build in your host machine or leverage docker to achieve this. 

The setup process roughly includes steps as follows:

1. Tool: Rust, PostgreSQL. We need you to create PostgreSQL with account `postgres` and password `postgres`, we'll use this account and DB for further data analysis. (TODO: Not support customization now.) While installing, you may face trouble. Refer following websites for help:
2. Import data and compile code.
   1. Go to the website [crates.io](https://crates.io/data-access) and follow Step 2 to download the ecosystem metadata. Pay attention to the `README.md` in the gz package, which tells you how to set up your DB. You should setup your database called 'crates'.
   2. The directory `crates.io-index` points to the index of crates.io(run `make submodule`). The index of crates should be the same as our database. As a result, we need to change checkout the git commits to meet the time of our database. For example, we download crates.db named `2023-01-11-020041`, we need to checkout the commits to or near 2023-01-11 02:00:41. Refer to entry `database` and `setindex` in `Makefile` for how to achieve this.
   3. Import modified Rustc codes under the directory `rust` (run `make submodule`), and build Rustc through the guide in the README file. Find the Rustc target binary for later use.
3. Now you've set up your data and tools. You may need further environment setup under specific projects, which is also detailedly described in the directory `./Code`. Refer to it for further analysis.

### Reproduce Our Results

Make sure that your processor(6+ cores), memory size(16+GB) and disk size (5GB for database maintenance, 50+GB for build, 1TB for ecosystem source code) are powerful enough, as we need to compile Rust compiler and analyze the entire ecosystem. 
Refer to [postgresql guide](#quick-guide-to-postgresql) section if you are not familiar with the Postgresql usage.

#### Using Ecosystem Raw Data

Considering the complexity of our tools, we provide existing ecosystem raw data (2022-08-11) generated by our tools. After reconstructing the database, you can view full results in the database and can query using SQL scripts from us (`./Code/scripts`) to reproduce results or even further analyze the ecosystem for your own research purposes.

Download Docker (https://www.docker.com/get-started/) and run commands:

```Shell
# Clone git submodule
make submodule
# Build Docker
docker build -t cargo-ecosystem-monitor .
# Run Docker 
docker run -it -dp 127.0.0.1:12345:5432 -e POSTGRES_PASSWORD="postgres" -w /app --mount type=bind,src="$(pwd)",target=/app cargo-ecosystem-monitor bash

# Exec into the docker shell
docker exec -it <docker-id> bash
# Setup Extra Confiigurations
make postgresql

# Download and import Raw Data (Password "postgres", Only Once). It may take 10min or more.
make download_20220811_rawdata
make import_20220811_rawdata
# Now, you can feel free to analyze the Rust ecosystem using postgresql scripts.
```
After setting all the runtime environment and dependencies, you can access to our sql scripts to investigate our database, or evaluate our tools and data.

You can access the postgresql from port `12345` in the host machine as user `postgres` with password `postgres`.

For **direct access** of our ecosystem-scale study results, you can refer to our sql file in `./Code/scripts/research_results.sql`, which looks like this:

```sql
-- Result 2: RUF Usage
-- RUF count
SELECT status, COUNT(*) FROM feature_status
  WHERE name in (SELECT DISTINCT feature FROM version_feature_ori)
  GROUP BY status;
-- RUF count (Complete)
SELECT name, status FROM feature_status
  WHERE name in (SELECT DISTINCT feature FROM version_feature_ori);
-- Package Versions
SELECT status, COUNT(DISTINCT id) FROM version_feature_ori INNER JOIN feature_status ON name=feature GROUP BY status;
-- RUF Usage Items
SELECT status, COUNT(*) FROM version_feature_ori INNER JOIN feature_status ON name=feature GROUP BY status; 
-- Package Versions + RUF Usage Items (Complete)
SELECT version_feature_ori.*, status FROM version_feature_ori INNER JOIN feature_status ON name=feature;
```

For example, if you want to replicate our results in RQ2 (RUF Usage) in our paper, You can use this command in the postgresql to query the RUF Usage Items:

```sql
-- RUF Usage Items
SELECT status, COUNT(*) FROM version_feature_ori INNER JOIN feature_status ON name=feature GROUP BY status; 
```




#### Using Ecosystem Metadata

The entire process may take 2 days as we need to resolve the ecosystem dependency graph and download all source codes of the ecosystem. To build our projects and generate raw data from scratch, you should do something more. First, you should download [ecosystem metadata 2022-08-11](https://drive.google.com/file/d/1-2oamGvhUOT4fIJlYB2e8PN9D_thHcmK/view?usp=sharing) under `./data` directory. The command lines should be like:

```Shell
# Clone git submodule
make submodule
# Build Docker
docker build -t cargo-ecosystem-monitor .
# Run Docker
docker run -it -dp 127.0.0.1:12345:5432 -e POSTGRES_PASSWORD="postgres" -w /app --mount type=bind,src="$(pwd)",target=/app cargo-ecosystem-monitor bash

# Exec into the docker shell
docker exec -it <docker-id> bash
# Setup Extra Confiigurations
make postgresql
# Extract Metadata (Only Once)
make extract_cratesio_20220811
# Import Metadata (Password "postgres", Only Once)
make database
# Now, you can feel free to other README and build tools. 
```

You can access the postgresql database `crates` from port `12345` in the host machine as user `postgres` with password `postgres`.


## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
After setting all the runtime environment and dependencies, you can access to our tools based on the README files. Refer to the sub-project overview [readme](./Code/README.md) file](./Code/README.md) for detailed usage guidance.


## Appendix

In case you are not familiar with some concepts mentioned in the documents. 

#### How is Our Data Generated?

Our data are generated through roughly three steps:

1. Rust **ecosystem metadata** from Rust official database.
2. **Ecosystem raw data**: The RUF extractor and ecosystem dependency generator tools will resolve metadata and produce raw data that drives our research topics. The raw data is stored in the database.
3. **Research results**: To finally reveal RUF findings and answer RQs, we further analyze the raw data using our scripts (mostly SQL scripts) and present them in the paper.

#### Research Beyond RUF (Extensibility)

Beyond the RUF study, our tools, scripts, and data can be extended for further research topics and applications.

1. Vulnerability Detection and Propagation: We have collected vulnerability metadata and bound them with the Rust Ecosystem Graph to identify the vulnerability impact in the ecosystem level. We found that due to the centralization of the Rust ecosystem, the vulnerability impact becomes very huge. The related tools are included in our source code.
2. Ecosystem Analysis in Other Dimensions: Super-spreaders of maintainers and packages are further examined in our analysis. We found that there are some maintainers and packages that can impact a wide variety of packages in the ecosystem. As the recent news shows that the Rust teams are not so stable, the super-spreader maintainers (lots are from the Rust team) may be a "weak point" of the ecosystem. Also, we find that some packages are not updated for years, but still own millions of downloads each year. Our ecosystem-level analyzer and database can be used to find similar findings easily.
3. Dependency Conflict: During the Ecosystem Dependency Graph generation process, we discovered that many packages are suffering from dependency resolution failure due to yanked packages or improper dependency configurations. We have analyzed some of them manually and found that the dependency conflict can be somehow recovered by modifying the dependency configurations. Moreover, we discovered some unstable dependency configurations that can easily cause packages to fail the dependency resolution. More dependency resolution findings are under research.

The extensibility of our technique reveals the significance of our proposed new technique, and we hope researchers can make benefit of our open-source data and tools to analyze the Rust ecosystem and the Rust compiler.


#### Quick Guide to Postgresql

We recommend three ways to use Postgresql.

1. Shell
   - Postgresql supports shell commands using `psql`. For example, to access our research results, you can use the command `psql -U postgres -d crates` to enter database `crates` as user `postgres`.
    
    ```Shell
    > psql -U postgres -d crates
    Password for user postgres: # Type your passwoard (default `postgres`)
    psql (15.4 (Ubuntu 15.4-1.pgdg20.04+1), server 14.9 (Ubuntu 14.9-1.pgdg20.04+1))
    Type "help" for help.

    crates=# 
    ```

    - Now you can type your Postgresql commands to access the database. For example, if you want to search for the latest RUF status. You can run the command below after entering the database.

    ```
    crates=# SELECT count(*), v1_63_0 as status FROM feature_timeline GROUP BY v1_63_0;
    count |   status   
    -------+------------
      241 | 
      562 | active
       11 | incomplete
     1002 | accepted
       59 | removed
    (5 rows)
    ```
2. VSCode Plugin
   - Search for the plugin `PostgreSQL` in your VSCode, and add connection to our database. Follow the instructions given by the plugin, and you can access and modify our database in your VSCode.
3. Desktop APP
   - You can use desktop APPs like `pgAdmin` to manage your Postgresql.


#### Common Build Problems

1. If you find that you can't access the [postgresql](https://stackoverflow.com/questions/55038942/fatal-password-authentication-failed-for-user-postgres-postgresql-11-with-pg) with "authentication failed", you need some steps to fix this:
      1. As we want to use database user "postgres", we need to access it in the database first. The postgresql will automatically create linux user "postgres", and we can only access database owned by "postgres" by switching to linux user "postgres". After switch, we modify the database user "postgres" password to "postgres" by command `su postgres -c "psql -c \"ALTER USER postgres PASSWORD 'postgres'\""`.
      2. As we want to access it whoever the linux users are, we need to change rules in `/etc/postgresql/$(postgresql_version)/main/pg_hba.conf` to use `md5` protocal. After rule changes, we can access the database of "postgres" using password "postgres".
      3. Restart the postgresql to update the change by command `service postgresql restart`.
   2. https://forge.rust-lang.org/infra/other-installation-methods.html
