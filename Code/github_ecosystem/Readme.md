# RUF Usage in Popular Github Projects

Beyond Cargo projects, we decide to move on to Rust projects in the Github to explore their usage. These projects are generally larger and contain workspaces.

### Quick Start

Run python scripts to download and analyze top 100 Rust projects (by starts) in the Github.

```Shell
python3 ruf_usage_hot_rust_projects.py
```

We also do case study on three influential projects (Rust for Linux, Android Open Source Project, Firefox) and analyze Rust codes in them.


#### Android

AOSP relies on tool `repo`. You can refer to instructions in https://source.android.com/docs/setup/download/downloading.

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

You can edit file `xxx/bin/repo` to change `#!/usr/bin/env python` to `#!/usr/bin/env python3`. After that, follow the instructions in https://source.android.com/docs/setup/download/downloading. Be sure to download under directory `./AOSP`.

#### Rust for Linux

Rust for Linux maintains an issue for Rust unstable features, where you can directly view all of them and history (click `edited`) in https://github.com/Rust-for-Linux/linux/issues/2.

However, as Rust for Linux requires modification to standard library (for low-level control like memory management), it also contains modified standard library source codes. Unfortunately, this introduces large amounts of RUF usage. Standard library is specifically designed for specific compiler, and this makes maintainance even harder.

```Shell
mkdir Rust4Linux
cd Rust4Linux
git clone https://github.com/Rust-for-Linux/linux.git
```


#### Firefox

```Shell
mkdir Firefox
cd Firefox
curl https://hg.mozilla.org/mozilla-central/raw-file/default/python/mozboot/bin/bootstrap.py -O
python3 bootstrap.py
```