use ansi_term::Color;
use clap::Parser;

mod feature;
mod rustc_version;
mod util;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long)]
    features: Vec<String>,
}

fn main() {
    match util::run(Cli::parse()) {
        Err(e) => {
            // fix failed
            println!("{} {}", Color::Red.paint("[Failed]"), e.to_string());
        }
        Ok(done) => {
            // feature fix done, but may be other errors
            if done {
                println!(
                    "{} {}",
                    Color::Green.paint("[Success]"),
                    "All ruf issues are fixed successfully."
                );
            } else {
                println!(
                    "{} {}",
                    Color::Yellow.paint("[Warn]"),
                    "Detected ruf issues were fixed, but there could still be some other errors that caused the compilation to fail. You can run `cargo build` locally to find out what's wrong."
                );
            }
        }
    }
}
