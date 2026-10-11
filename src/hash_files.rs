use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

use blake3::{Hash, Hasher};
use dashmap::DashMap;
use ignore::WalkBuilder;
use rayon::iter::{ParallelBridge, ParallelIterator};
use size::Size;

use crate::{
    Options,
    file_cache::{FileCache, FileCacheMap},
};

pub struct HashedFilePaths(HashMap<Hash, FileInfo>);

impl DerefMut for HashedFilePaths {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Deref for HashedFilePaths {
    type Target = HashMap<Hash, FileInfo>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

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
    ) -> (Self, Size) {
        let paths: DashMap<Hash, FileInfo> = DashMap::new();
        let cache_ref = &cache.as_ref();

        let new_cache = WalkBuilder::new(root)
            .hidden(options.ignore_hidden)
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

                let path = entry.path();

                // Ignore if path is a directory, symlink, or readonly.
                // We ignore symlinks here as they take up no space and would not be useful to be
                // reported.
                // We ignore read only by default as we cannot clean them up.
                if path.is_dir()
                    || entry.path_is_symlink()
                    || entry.metadata().is_ok_and(|m| m.permissions().readonly())
                {
                    return None;
                }

                if let Some(cache) = cache_ref
                    && let Some(cached) = cache.get(path)
                    && let Ok(metadata) = entry.metadata()
                    && let Ok(modified) = metadata.modified()
                    && cached.modified() == modified
                {
                    if let Err(e) = paths.entry(cached.hash()).or_default().push(path) {
                        eprintln!("{path:?}: {e}");
                    }
                    None
                } else {
                    let cached = match FileCache::new(path) {
                        Ok(cached) => cached,
                        Err(e) => {
                            eprintln!("{path:?}: {e}");
                            return None;
                        }
                    };
                    if let Err(e) = paths.entry(cached.hash()).or_default().push(path) {
                        eprintln!("{path:?}: {e}");
                        return None;
                    }
                    Some((path.to_path_buf(), cached))
                }
            })
            .collect::<HashMap<PathBuf, FileCache>>();

        // Convert to HashMap and filter out all non duplicated.
        let paths: HashMap<Hash, FileInfo> = paths
            .into_iter()
            .filter(|(_, item)| item.paths.len() > 1)
            .collect();

        let total_checked_size = new_cache
            .iter()
            .fold(Size::from_bytes(0), |acc, (_, item)| {
                acc + Size::from_bytes(item.size())
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
    size: Option<Size>,
}
impl FileInfo {
    fn push(&mut self, p: impl AsRef<Path>) -> anyhow::Result<()> {
        if self.size.is_none() {
            self.size = Some(Size::from_bytes(p.as_ref().metadata()?.len()));
        }
        self.paths.push(p.as_ref().to_path_buf());
        Ok(())
    }
    pub fn size(&self) -> Size {
        self.size.unwrap_or_default()
    }
    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }
}
