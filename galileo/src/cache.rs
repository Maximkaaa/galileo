use bytes::Bytes;
use log::info;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FileCacheController {
    folder_path: PathBuf,
}

impl FileCacheController {
    pub fn new() -> Self {
        let folder_path = get_folder_path();
        ensure_folder_exists(&folder_path).unwrap();
        Self { folder_path }
    }

    pub fn get_from_cache(&self, url: &str) -> Option<Bytes> {
        let file_path = self.get_file_path(url);
        if let Ok(bytes) = std::fs::read(file_path) {
            Some(bytes.into())
        } else {
            None
        }
    }

    fn get_file_path(&self, url: &str) -> PathBuf {
        let stripped = if url.starts_with("http://") {
            &url[7..]
        } else if url.starts_with("https://") {
            &url[8..]
        } else {
            &url
        };

        self.folder_path.join(Path::new(stripped))
    }

    pub fn save_to_cache(&self, url: &str, bytes: &Bytes) {
        let file_path = self.get_file_path(url);
        ensure_folder_exists(&file_path.parent().unwrap()).unwrap();
        match std::fs::write(&file_path, &bytes) {
            Ok(_) => info!("Url {url} saved to cache file {file_path:?}"),
            Err(e) => info!("Failed to save {url} to cache: {e:?}"),
        };
    }
}

fn ensure_folder_exists(folder_path: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(folder_path)
}

const CACHE_FOLDER: &str = ".tile_cache";

fn get_folder_path() -> PathBuf {
    Path::new(CACHE_FOLDER).into()
}
