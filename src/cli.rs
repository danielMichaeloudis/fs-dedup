use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
pub struct Cli {
    #[arg(default_value = "/")]
    pub path: PathBuf,
    #[arg(long, action)]
    pub no_cache: bool,
    #[arg(short, long, action)]
    pub show_hidden: bool,
}
