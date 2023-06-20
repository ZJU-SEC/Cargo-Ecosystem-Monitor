use downloader::{Download, Downloader};
use flate2::read::GzDecoder;
use lazy_static::lazy_static;
use postgres::Client;
use regex::Regex;
use tar::Archive;
use walkdir::WalkDir;

use std::collections::{HashMap, HashSet};
use std::fs::{remove_dir_all,create_dir, File, OpenOptions};
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct Feature {
    pub name: String,
    pub status: Status,
    pub rustc_const: bool,
}

pub struct Features {
    pub lang: Vec<Feature>,
    pub lib: Vec<Feature>,
    pub mul_errors: HashSet<String>,
    pub nr_errors: HashSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Active,
    Accepted,
    Incomplete,
    Removed,
}

#[allow(unused)]
pub fn download_info() {
    remove_dir_all("on_process").unwrap_or_default();
    create_dir("on_process").unwrap_or_default();

    let mut downloader = Downloader::builder()
        .download_folder(Path::new("on_process"))
        .parallel_requests(4)
        .build()
        .expect("Fatal Error, build downloader fails!");

    let mut dls = Vec::new();

    for ver in 0..=67 {
        dls.push(Download::new(&format!(
            "https://github.com/rust-lang/rust/archive/refs/tags/1.{ver}.0.tar.gz",
        )));
    }

    downloader.download(&dls).expect("downloader broken");
}

pub fn extract_info(ver: i32) -> Features {
    let ver_str = format!("1.{}.0", ver);
    let data = File::open(&format!("on_process/{}.tar.gz", ver_str)).expect("Open file failed");
    let mut archive = Archive::new(GzDecoder::new(data));

    if File::open(&format!("on_process/rust-{}", ver_str)).is_err() {
        println!("Unpacked {}", ver_str);
        archive.unpack("on_process/").expect("Unpack file failed");
    }

    println!("Processing {}", ver_str);

    let lang_feature = extract_lang_feature(ver);
    let (lib_feature, mul, nr) = extract_lib_feature(ver);

    Features {
        lang: lang_feature,
        lib: lib_feature,
        mul_errors: mul,
        nr_errors: nr,
    }
}

fn extract_lib_feature(ver: i32) -> (Vec<Feature>, HashSet<String>, HashSet<String>) {
    let mut lib_features = HashMap::new();
    let mut mul_status = HashSet::new();
    let mut nr_error = HashSet::new();

    let ver_str = format!("1.{}.0", ver);
    let dir = WalkDir::new(format!("on_process/rust-{}", ver_str)).into_iter();

    for entry in dir.filter_entry(|e| !e.file_name().to_str().unwrap().contains("test")) {
        let entry = entry.unwrap();
        let file = entry.path();
        let filename = file.file_name().unwrap().to_string_lossy();
        if !filename.ends_with(".rs")
            || filename == "features.rs"
            || filename == "diagnostic_list.rs"
            || filename == "error_codes.rs"
        {
            continue;
        }

        let mut file = File::open(entry.path()).expect("Open file failed");
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .expect("Read file failed");

        map_lib_features(contents, &mut |res| {
            if let Ok(feature) = res {
                let f = feature.clone();
                let entry = lib_features.entry(feature.name.clone()).or_insert(feature);
                // Inconsistent status
                if entry.status != f.status {
                    entry.status = Status::Active;
                    mul_status.insert(f.name.clone());
                }
                // Inconsistent rustc_const
                if entry.rustc_const != f.rustc_const {
                    entry.rustc_const = true;
                    nr_error.insert(f.name);
                }
            }
        });
    }

    (lib_features.drain().into_iter().map(|(_, v)| v).collect(), mul_status, nr_error)
}

fn extract_lang_feature(ver: i32) -> Vec<Feature> {
    let ver_str = format!("1.{}.0", ver);
    let mut features = Vec::new();

    lazy_static! {
        static ref RE1: Regex =
            Regex::new(r#"\("([a-zA-Z0-9_-]+?)", .+, (Active|Accepted|Removed|Incomplete)\)"#)
                .unwrap();
        static ref RE2: Regex =
            Regex::new(r#"\((active|accepted|removed|incomplete), ([a-zA-Z0-9_-]+?), .+\)"#)
                .unwrap();
    }

    match ver {
        0..=38 => {
            let mut file = OpenOptions::new()
                .read(true)
                .open(&format!(
                    "on_process/rust-{}/src/libsyntax/feature_gate.rs",
                    ver_str
                ))
                .expect("Open file failed");

            let mut content = String::new();
            file.read_to_string(&mut content).expect("Read file failed");

            if ver <= 9 {
                for cap in RE1.captures_iter(&content) {
                    features.push(Feature {
                        name: cap[1].to_string(),
                        status: cap[2].to_ascii_lowercase().into(),
                        rustc_const: false,
                    });
                }
            } else {
                for cap in RE2.captures_iter(&content) {
                    features.push(Feature {
                        name: cap[2].to_string(),
                        status: cap[1].to_ascii_lowercase().into(),
                        rustc_const: false,
                    });
                }
            }
        }
        39..=67 => {
            let pre_path = match ver {
                39..=40 => "src/libsyntax/feature_gate",
                41..=47 => "src/librustc_feature",
                48..=67 => "compiler/rustc_feature/src",
                _ => unreachable!(),
            };

            let mut content = String::new();

            for name in ["accepted.rs", "active.rs", "removed.rs"] {
                let mut file = OpenOptions::new()
                    .read(true)
                    .open(&format!(
                        "on_process/rust-{}/{}/{}",
                        ver_str, pre_path, name
                    ))
                    .expect("Open file failed");

                let mut buf = String::new();
                file.read_to_string(&mut buf).expect("Read file failed");
                content.push_str(&buf);
            }

            for cap in RE2.captures_iter(&content) {
                features.push(Feature {
                    name: cap[2].to_string(),
                    status: cap[1].to_ascii_lowercase().into(),
                    rustc_const: false,
                });
            }
        }
        _ => unreachable!(),
    }

    features
}

fn map_lib_features(contents: String, mf: &mut dyn FnMut(Result<Feature, &str>)) {
    // This is an early exit -- all the attributes we're concerned with must contain this:
    // * rustc_const_unstable(
    // * rustc_const_stable(
    // * unstable(
    // * stable(
    if !contents.contains("stable(") {
        return;
    }

    let mut becoming_feature: Option<Feature> = None;
    let mut iter_lines = contents.lines().enumerate().peekable();
    while let Some((_, line)) = iter_lines.next() {
        macro_rules! err {
            ($msg:expr) => {{
                mf(Err($msg));
                continue;
            }};
        }

        lazy_static::lazy_static! {
            static ref COMMENT_LINE: Regex = Regex::new(r"^\s*//").unwrap();
        }
        // exclude commented out lines
        if COMMENT_LINE.is_match(line) {
            continue;
        }

        if let Some(ref mut f) = becoming_feature {
            if line.ends_with(']') {
                mf(Ok(f.clone()));
            } else if !line.ends_with(',') && !line.ends_with('\\') && !line.ends_with('"') {
                // We need to bail here because we might have missed the
                // end of a stability attribute above because the ']'
                // might not have been at the end of the line.
                // We could then get into the very unfortunate situation that
                // we continue parsing the file assuming the current stability
                // attribute has not ended, and ignoring possible feature
                // attributes in the process.
                err!("malformed stability attribute");
            } else {
                continue;
            }
        }

        becoming_feature = None;
        if line.contains("rustc_const_unstable(") {
            // `const fn` features are handled specially.
            let feature_name = match find_attr_val(line, "feature").or_else(|| {
                iter_lines
                    .peek()
                    .and_then(|next| find_attr_val(next.1, "feature"))
            }) {
                Some(name) => name,
                None => err!("malformed stability attribute: missing `feature` key"),
            };
            let feature = Feature {
                name: feature_name.to_string(),
                status: Status::Active,
                rustc_const: true,
            };
            mf(Ok(feature));
            continue;
        }

        if line.contains("rustc_const_stable(") {
            // `const fn` features are handled specially.
            let feature_name = match find_attr_val(line, "feature").or_else(|| {
                iter_lines
                    .peek()
                    .and_then(|next| find_attr_val(next.1, "feature"))
            }) {
                Some(name) => name,
                None => err!("malformed stability attribute: missing `feature` key"),
            };
            let feature = Feature {
                name: feature_name.to_string(),
                status: Status::Accepted,
                rustc_const: true,
            };
            mf(Ok(feature));
            continue;
        }

        let level = if line.contains("[unstable(") {
            Status::Active
        } else if line.contains("[stable(") {
            Status::Accepted
        } else {
            continue;
        };

        let feature_name = match find_attr_val(line, "feature").or_else(|| {
            iter_lines
                .peek()
                .and_then(|next| find_attr_val(next.1, "feature"))
        }) {
            Some(name) => name,
            None => err!("malformed stability attribute: missing `feature` key"),
        };

        let feature = Feature {
            name: feature_name.to_string(),
            status: level,
            rustc_const: false,
        };

        if line.contains(']') {
            mf(Ok(feature));
        } else {
            becoming_feature = Some(feature);
        }
    }
}

fn find_attr_val<'a>(line: &'a str, attr: &str) -> Option<&'a str> {
    lazy_static::lazy_static! {
        static ref FEATURE: Regex = Regex::new(r#"feature\s*=\s*"([a-zA-Z0-9_-]+)""#).unwrap();
    }

    let r = match attr {
        "feature" => &*FEATURE,
        _ => unimplemented!("{attr} not handled"),
    };

    r.captures(line).and_then(|c| c.get(1)).map(|m| m.as_str())
}

impl From<String> for Status {
    fn from(s: String) -> Self {
        match s.as_str() {
            "active" => Status::Active,
            "accepted" => Status::Accepted,
            "removed" => Status::Removed,
            "incomplete" => Status::Incomplete,
            _ => unreachable!(),
        }
    }
}

impl ToString for Status {
    fn to_string(&self) -> String {
        match self {
            Status::Active => "active",
            Status::Accepted => "accepted",
            Status::Removed => "removed",
            Status::Incomplete => "incomplete",
        }
        .to_string()
    }
}

pub fn prebuild(conn: Arc<Mutex<Client>>) {
    conn.lock()
        .unwrap()
        .query(
            r#"CREATE TABLE IF NOT EXISTS public.feature_timeline (name VARCHAR)"#,
            &[],
        )
        .unwrap();
}
