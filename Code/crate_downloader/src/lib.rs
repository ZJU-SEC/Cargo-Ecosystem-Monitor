use std::fs::{create_dir};
use std::io;
use std::path::Path;
use std::process::{Command, Output};


use anyhow::{anyhow, Result};
use downloader::{Download, Downloader};


pub fn fetch_crate(downloader: &mut Downloader, store_path: &str, name: &str, version: &str) -> Result<()> {
    let mut dls = vec![];

    create_dir(Path::new(&format!("{}/{}", store_path, name))).unwrap_or_default();

    dls.push(
        Download::new(&format!(
            "https://crates.io/api/v1/crates/{name}/{version}/download",
        ))
        .file_name(Path::new(&format!("{name}/{version}.tgz"))),
    );

    let res = downloader.download(&dls)?;

    if res.first().unwrap().is_err() {
        return Err(anyhow!("Download error"));
    }

    return Ok(());
}

pub fn deal_with_crate(store_path: &str, name: &str, version: &str) -> io::Result<Output> {
    // Decompress
    Command::new("tar")
        .arg("-zxf")
        .arg(format!("{store_path}/{name}/{version}.tgz"))
        .arg("-C")
        .arg(format!("{store_path}/{name}"))
        .output()
}