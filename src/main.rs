use std::time::Instant;

use clap::Parser;
use size::Size;

use crate::{cli::Cli, file_cache::FileCacheMap, hash_files::HashedFilePaths};

mod cli;
mod config;
mod consts;
mod file_cache;
mod hash_files;

#[derive(Debug, Default)]
pub struct Options {
    pub ignore_hidden: bool,
}

impl From<&Cli> for Options {
    fn from(value: &Cli) -> Self {
        Self {
            ignore_hidden: !value.show_hidden,
        }
    }
}

fn main() -> Result<(), anyhow::Error> {
    let args = Cli::parse();

    if !args.path.exists() {
        anyhow::bail!("Path does not exist");
    }
    if !args.path.is_dir() {
        anyhow::bail!("Path is not a directory")
    }

    let options = Options::from(&args);

    let mut cache = if !args.no_cache {
        Some(FileCacheMap::get())
    } else {
        None
    };

    let start = Instant::now();

    let (files, size) = HashedFilePaths::load_from_root(&args.path, options, cache.as_mut());

    let end = Instant::now();

    if let Some(cache) = cache {
        // We can ignore errors in saving as it is not too important that cache is saved.
        let _ = cache.save();
    }

    let savable = files.values().fold(Size::from_bytes(0), |acc, item| {
        let mut names = item
            .paths()
            .iter()
            .filter_map(|item| item.file_name())
            .collect::<Vec<_>>();
        names.dedup();
        let dups = item.paths().len();
        let savable = item.size() * (dups - 1);
        println!("Found {dups} copies of files {names:?} at {savable}");
        acc + savable
    });

    println!("Took {:?} to read {}", end - start, size);

    println!("Removing duplicates will save {savable}");

    Ok(())
}
