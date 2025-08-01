use std::path::{Path, PathBuf};

use bytes::Bytes;
use log::debug;

use crate::error::GalileoError;
use crate::layer::data_provider::PersistentCacheController;

/// Function to modify the default file path of the cache
pub type FileCachePathModifier = dyn Fn(&str) -> String + Send + Sync;

/// Modifier to remove parameters from file path.
/// Can be used as a [`FileCachePathModifier`].
pub fn remove_parameters_modifier(path: &str) -> String {
    path.split('?').next().unwrap_or(path).to_owned()
}

/// Stores the cached data as a set of files in the specified folder. It generates file names from the given urls.
///
/// Currently, there is no eviction mechanism.
pub struct FileCacheController {
    folder_path: PathBuf,
    /// Function to modify the default file path of the cache (optional)
    file_path_modifier: Option<Box<FileCachePathModifier>>,
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
    /// it will be created on startup. In this directory each tile will be stored in a nested folder
    /// based on the original url of that tile. The structure of those nested folders can be modified
    /// by `file_path_modifier`. Check [`FileCacheController::get_file_path`] for details.
    pub fn new(
        path: impl AsRef<Path>,
        file_path_modifier: Option<Box<FileCachePathModifier>>,
    ) -> Result<Self, GalileoError> {
        ensure_folder_exists(path.as_ref()).map_err(|err| {
            GalileoError::FsIo(format!(
                "failed to initialize file cache folder {:?}: {err}",
                path.as_ref()
            ))
        })?;
        Ok(Self {
            folder_path: path.as_ref().into(),
            file_path_modifier,
        })
    }

    fn get_file_path(&self, url: &str) -> PathBuf {
        let stripped = if let Some(v) = url.strip_prefix("http://") {
            v
        } else if let Some(v) = url.strip_prefix("https://") {
            v
        } else {
            url
        };

        let path = if let Some(modifier) = &self.file_path_modifier {
            modifier(stripped)
        } else {
            stripped.to_string()
        };

        self.folder_path.join(Path::new(&path))
    }
}

fn ensure_folder_exists(folder_path: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(folder_path)
}
