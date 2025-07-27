use std::path::{Path, PathBuf};

use bytes::Bytes;
use log::debug;

use crate::error::GalileoError;
use crate::layer::data_provider::PersistentCacheController;

/// Stores the cached data as a set of files in the specified folder. It generates file names from the given urls.
///
/// Currently, there is no eviction mechanism.
#[derive(Debug, Clone)]
pub struct FileCacheController {
    folder_path: PathBuf,
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
    pub fn new(path: impl AsRef<Path>) -> Result<Self, GalileoError> {
        ensure_folder_exists(path.as_ref()).map_err(|err| {
            GalileoError::FsIo(format!(
                "failed to initialize file cache folder {:?}: {err}",
                path.as_ref()
            ))
        })?;
        Ok(Self {
            folder_path: path.as_ref().into(),
        })
    }

    fn get_file_path(&self, url: &str) -> PathBuf {
        let path = path_from_url(url);
        self.folder_path.join(path)
    }
}

fn ensure_folder_exists(folder_path: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(folder_path)
}

fn path_from_url(url: &str) -> &Path {
    // strip `http` or `https` from url
    let path = ["http://", "https://"]
        .iter()
        .find_map(|prefix| url.strip_prefix(prefix))
        .unwrap_or(url)
        // strip query parameters if any
        .split('?')
        .next()
        .unwrap_or(url);
    Path::new(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_path_from_url() {
        let path1 = path_from_url("http://api.maptiler.com/tiles/id/1/2/3.png");
        let path2 = path_from_url("https://api.maptiler.com/tiles/id/1/2/3.png");
        let path3 = path_from_url("http://api.maptiler.com/tiles/id/1/2/3.png?key=abc");
        let path4 = path_from_url("https://api.maptiler.com/tiles/id/1/2/3.png?key=abc");

        let expected = Path::new("api.maptiler.com/tiles/id/1/2/3.png");
        assert_eq!(expected, path1);
        assert_eq!(expected, path2);
        assert_eq!(expected, path3);
        assert_eq!(expected, path4);
    }
}
