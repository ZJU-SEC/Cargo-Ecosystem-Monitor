use clap::{Parser};

mod util;
mod feature;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long)]
    features: Vec<String>,
}

fn main() {
    util::run(Cli::parse());
}