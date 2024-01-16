# RUF Usage in Popular Github Projects

Beyond Cargo projects, we decide to move on to Rust projects in GitHub to explore their usage. These projects are generally larger and contain workspaces.

### Quick Start

Run Python scripts to download and analyze the top 100 Rust projects (by starts) in GitHub.

```Shell
# download
python3 ruf_usage_hot_rust_projects.py download_hot_projects
# analyze ruf usage of the projects
python3 ruf_usage_hot_rust_projects.py ruf_usage
# configure your cargo config and run
python3 ruf_usage_hot_rust_projects.py ruf_impacts
# remove cargo config
```

To make our results reproducible, we changed crates.io index to a fixed version (2022-08-11). You have to configure it before running ruf_impacts, and remove them after running successfully. Please backup your cargo config in `~/.cargo/Cargo.toml` first. After running ruf_impacts, you should restore your config.


1. Override configuration to file `~/.cargo/config.toml` with 
```Rust
[net]
git-fetch-with-cli = true

[source.cargo_ecosystem_monitor]
registry = "file:///absolute/path/to/crates.io-index/dir" 

[source.crates-io]
replace-with = "cargo_ecosystem_monitor"
```
2. If you are using the provided docker, you can directly run `make replace_cargo_mirror` before running the evaluation process. And after the evaluation process, run `make restore_cargo_mirror` to remove the configurations. Make sure you know what is going to happen when you run it in your host machine.

```Shell
# Configure
make replace_cargo_mirror
# Remove
make restore_cargo_mirror
```



We also do case studies on three influential projects (Rust for Linux, Android Open Source Project, Firefox) and analyze Rust codes in them. You need to first download them and then run the scripts below.

```Shell
python3 ruf_usage_hot_rust_projects.py case_study
```


### Case Study

We analyze three influential projects that contain Rust code, Android Open Source Project (AOSP), Firefox, Rust for Linux.


#### Android

AOSP relies on tool `repo`. You can refer to the instructions at https://source.android.com/docs/setup/download/downloading.

```Shell
export REPO=$(mktemp /tmp/repo.XXXXXXXXX)
curl -o ${REPO} https://storage.googleapis.com/git-repo-downloads/repo
gpg --recv-keys 8BB9AD793E8E6153AF0F9A4416530D5E920F5C65
curl -s https://storage.googleapis.com/git-repo-downloads/repo.asc | gpg --verify - ${REPO} && install -m 755 ${REPO} ~/bin/repo
```

If you see errors like this:

```
File "/home/loancold/bin/repo", line 51
def print(self, *args, **kwargs):
        ^
SyntaxError: invalid syntax
```

You can edit file `xxx/bin/repo` to change `#!/usr/bin/env python` to `#!/usr/bin/env python3`. After that, follow the instructions at https://source.android.com/docs/setup/download/downloading. Be sure to download it under the directory `./AOSP`.

Under directory `./AOSP`, run command

```Shell
repo forall -v -c 'git checkout `git rev-list -n 1 --first-parent --before="2022-08-11 00:00:00 +0000" HEAD`'
```

#### Rust for Linux

Rust for Linux maintains an issue for Rust unstable features, where you can directly view all of them and the history (click `edited`) at https://github.com/Rust-for-Linux/linux/issues/2.

However, as Rust for Linux requires modification to the standard library (for low-level control like memory management), it also contains modified standard library source codes. Unfortunately, this introduces large amounts of RUF usage. The standard library is specifically designed for specific compilers, and this makes maintenance even harder.

```Shell
mkdir Rust4Linux
cd Rust4Linux
git clone https://github.com/Rust-for-Linux/linux.git
cd linux
git checkout `git rev-list -n 1 --first-parent --before="2022-08-11 00:00:00 +0000" origin/rust`
```


#### Firefox

```Shell
mkdir Firefox
cd Firefox
curl https://hg.mozilla.org/mozilla-central/raw-file/default/python/mozboot/bin/bootstrap.py -O
python3 bootstrap.py --vcs=git
cd mozilla-unified
git checkout `git rev-list -n 1 --first-parent --before="2022-08-11 00:00:00 +0000" origin/HEAD`
```
