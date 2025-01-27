use std::path::{Path, PathBuf};

use bytes::Bytes;
use log::debug;

use crate::error::GalileoError;
use crate::layer::data_provider::PersistentCacheController;

const CACHE_FOLDER: &str = ".tile_cache";

/// Stores the cached data as a set of files in the specified folder. It generates file names from the given urls.
///
/// Currently, there is no eviction mechanism.
#[derive(Debug, Clone)]
pub struct FileCacheController {
    folder_path: PathBuf,
}

impl Default for FileCacheController {
    fn default() -> Self {
        Self::new(CACHE_FOLDER)
    }
}

impl PersistentCacheController<str, Bytes> for FileCacheController {
    fn get(&self, key: &str) -> Option<Bytes> {
        let file_path = self.get_file_path(key);
        if let Ok(bytes) = std::fs::read(file_path) {
            Some(bytes.into())
        } else {
            None
        }
    }

    fn insert(&self, key: &str, data: &Bytes) -> Result<(), GalileoError> {
        let file_path = self.get_file_path(key);
        match file_path.parent() {
            Some(folder) => match ensure_folder_exists(folder) {
                Ok(()) => {
                    debug!("Saving entry {key} to the cache file {file_path:?}");
                    std::fs::write(&file_path, data)?;
                    debug!("Entry {key} saved to cache file {file_path:?}");
                    Ok(())
                }
                Err(err) => {
                    debug!("Failed to add {key} entry to the cache failed {file_path:?} - failed to create folder: {err:?}");
                    Err(err.into())
                }
            },
            None => {
                debug!(
                    "Failed to add {key} entry to the cache failed {file_path:?} - no parent folder"
                );
                Err(GalileoError::IO)
            }
        }
    }
}

impl FileCacheController {
    /// Creates a new instance. The cache will be located in the given directory. If the directory doesn't exist,
    /// it will be created on startup.
    pub fn new(path: impl AsRef<Path>) -> Self {
        ensure_folder_exists(path.as_ref()).expect("Failed to initialize file cache controller.");
        Self {
            folder_path: path.as_ref().into(),
        }
    }

    fn get_file_path(&self, url: &str) -> PathBuf {
        let stripped = if let Some(v) = url.strip_prefix("http://") {
            v
        } else if let Some(v) = url.strip_prefix("https://") {
            v
        } else {
            url
        };

        self.folder_path.join(Path::new(stripped))
    }
}

fn ensure_folder_exists(folder_path: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(folder_path)
}
