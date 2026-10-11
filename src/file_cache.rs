use std::{
    collections::HashMap,
    fs, io,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    time::SystemTime,
};

use blake3::Hash;
use serde::{Deserialize, Serialize};

use crate::{
    consts::{env::CACHE_DIR, fs::CACHE_FILE, APP_NAME},
    hash_files::file_hash,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct FileCache {
    last_modified_time: SystemTime,
    hash: Hash,
    size: u64,
}

impl FileCache {
    pub fn new(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let hash = file_hash(path)?;
        let size = path.metadata()?.len();
        Ok(Self {
            last_modified_time: path.metadata()?.modified()?,
            hash,
            size,
        })
    }

    pub fn modified(&self) -> SystemTime {
        self.last_modified_time
    }

    pub fn hash(&self) -> Hash {
        self.hash
    }

    pub fn size(&self) -> u64 {
        self.size
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct FileCacheMap(HashMap<PathBuf, FileCache>);
impl DerefMut for FileCacheMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Deref for FileCacheMap {
    type Target = HashMap<PathBuf, FileCache>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<HashMap<PathBuf, FileCache>> for FileCacheMap {
    fn from(value: HashMap<PathBuf, FileCache>) -> Self {
        Self(value)
    }
}

impl FileCacheMap {
    pub fn get() -> Self {
        let cache_path = match Self::get_cache_path() {
            Some(path) => path,
            None => return Self::default(),
        };
        println!("found cache path: {:?}", cache_path);

        let file = match fs::read(cache_path) {
            Ok(file) => file,
            Err(_) => return Self::default(),
        };

        serde_json::from_slice(&file).unwrap_or_default()
    }

    fn get_cache_path() -> Option<PathBuf> {
        Some(match std::env::var(CACHE_DIR) {
            Ok(dir) => PathBuf::from(dir).join(CACHE_FILE),
            Err(_) => dirs::cache_dir()?.join(APP_NAME).join(CACHE_FILE),
        })
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let cache_path =
            Self::get_cache_path().ok_or(anyhow::anyhow!("Cache directory does not exist."))?;
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(fs::write(cache_path, serde_json::to_string(self)?)?)
    }
}
