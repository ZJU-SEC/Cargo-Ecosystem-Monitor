# Advisory Scanner

Scan the advisory impact range across the Rust ecosystem according to provided advisory data in json file. Should be done after project `rust_deps`. This tool is not included in RUF study, but is extended for vulnerability study in the Rust ecosystem, and can reuse the exsisting architecture to achieve the goal.

It accepts two json file extracted from github and rustsec database (now updated in 2022-03), and analyze the vulnerability impact in the Rust ecosystem through transitive dependencies. The result is stored in file `results.txt`.