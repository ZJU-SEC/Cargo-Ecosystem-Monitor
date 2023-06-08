# RUF Impact Mitigation Analysis

In this project, we will use our RUF remediation technique to scan the whole Rust ecosystem to analyze how many packges can recover from RUF threats, like compilation failure.

### Technique

Basically, we will try to choose the compiler version that can support active RUF used by packages, rather than removed or unknown RUF. 

### Procedure

0. Prelimilaries: You have to first build DB table `tmp_ruf_remediation_analysis` that stores information of how RUF impacts on package versions.
1. Build RUF impact table <version, RUF>, where package version is impacted by RUF.
2. For each verion, we will scan every compiler version to see, whether package version can use active or stable RUF instead of removed or unknown RUF. That's because the removed or unknown RUF is once supported by Rust compiler.
3. After that, we will give the results on whether and how to remediate RUF threats in each package version.