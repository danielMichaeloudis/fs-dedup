pub const APP_NAME: &str = "fs_dedup";
pub mod env {
    pub const CONFIG_DIR: &str = "FS_DEDUP_CONFIG_DIR";
    pub const CACHE_DIR: &str = "FS_DEDUP_CACHE_DIR";
}

pub mod fs {
    pub const CACHE_FILE: &str = "fs_dedup_cache.json";
}
