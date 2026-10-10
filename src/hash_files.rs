use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use blake3::{Hash, Hasher};
use dashmap::DashMap;
use ignore::WalkBuilder;
use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::{
    Options,
    file_cache::{FileCache, FileCacheMap},
};

pub struct HashedFilePaths(HashMap<Hash, FileInfo>);

pub fn file_hash(path: impl AsRef<Path>) -> std::io::Result<Hash> {
    let mut hasher = Hasher::new();
    hasher.update_mmap(path)?;
    Ok(hasher.finalize())
}

impl HashedFilePaths {
    pub fn load_from_root(
        root: impl AsRef<Path>,
        options: Options,
        cache: Option<&mut FileCacheMap>,
    ) -> (Self, u64) {
        let paths: DashMap<Hash, FileInfo> = DashMap::new();
        let cache_ref = &cache.as_ref();

        let new_cache = WalkBuilder::new(root)
            .hidden(options.show_hidden)
            .build()
            .par_bridge()
            .filter_map(|entry| {
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        eprintln!("{e}");

                        return None;
                    }
                };
                if entry.path().is_dir() {
                    return None;
                }
                if let Some(cache) = cache_ref
                    && let Some(cached) = cache.get(entry.path())
                    && let Ok(metadata) = entry.metadata()
                    && let Ok(modified) = metadata.modified()
                    && cached.modified() == modified
                {
                    if let Err(e) = paths.entry(cached.hash()).or_default().push(entry.path()) {
                        eprintln!("{e}");
                    }
                    None
                } else {
                    //println!("Not found in cache: {:?}", cache_ref.unwrap().len());
                    let cached = match FileCache::new(entry.path()) {
                        Ok(cached) => cached,
                        Err(e) => {
                            eprintln!("{e}");
                            return None;
                        }
                    };
                    if let Err(e) = paths.entry(cached.hash()).or_default().push(entry.path()) {
                        eprintln!("{e}");
                        return None;
                    }
                    Some((entry.path().to_path_buf(), cached))
                }
            })
            .collect::<HashMap<PathBuf, FileCache>>();

        // Convert to HashMap and filter out all non duplicated.
        let paths: HashMap<Hash, FileInfo> = paths
            .into_iter()
            .filter(|(_, item)| !item.paths.is_empty())
            .collect();

        let total_checked_size = new_cache.iter().fold(0, |mut acc, (_, item)| {
            acc += item.size();
            acc
        });

        if let Some(cache) = cache {
            cache.extend(new_cache);
        }

        (Self(paths), total_checked_size)
    }
}

#[derive(Debug, Default)]
pub struct FileInfo {
    paths: Vec<PathBuf>,
    size: Option<u64>,
}
impl FileInfo {
    fn push(&mut self, p: impl AsRef<Path>) -> anyhow::Result<()> {
        if self.size.is_none() {
            self.size = Some(p.as_ref().metadata()?.len());
        }
        self.paths.push(p.as_ref().to_path_buf());
        Ok(())
    }
}
