use std::time::Instant;

use clap::Parser;

use crate::{cli::Cli, file_cache::FileCacheMap, hash_files::HashedFilePaths};

mod cli;
mod config;
mod consts;
mod file_cache;
mod hash_files;

#[derive(Debug, Default)]
pub struct Options {
    pub show_hidden: bool,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Cli::parse();

    if !args.path.exists() {
        anyhow::bail!("Path does not exist");
    }
    if !args.path.is_dir() {
        anyhow::bail!("Path is not a directory")
    }

    let options = Options::default();

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

    println!(
        "Took {:?} to read {}GB",
        end - start,
        size / (1024u64.pow(3))
    );

    Ok(())
}
