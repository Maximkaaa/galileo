use crate::cache::FileCacheController;
use crate::error::GalileoError;
use crate::platform::PlatformService;
use crate::primitives::DecodedImage;
use async_trait::async_trait;
use bytes::Bytes;
use log::info;

#[derive(Debug, Clone)]
pub struct NativePlatformService {
    http_client: reqwest::Client,
    cache_controller: FileCacheController,
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl PlatformService for NativePlatformService {
    fn new() -> Self {
        let http_client = reqwest::Client::builder()
            .user_agent("galileo/0.1")
            .build()
            .unwrap();
        let cache_controller = FileCacheController::new();

        Self {
            http_client,
            cache_controller,
        }
    }

    async fn load_image_url(&self, url: &str) -> Result<DecodedImage, GalileoError> {
        let image_source = match self.load_from_cache(url) {
            Some(v) => v,
            None => {
                let loaded = self.load_from_web(url).await?;
                self.cache_controller.save_to_cache(url, &loaded);
                loaded
            }
        };

        Ok(DecodedImage::new(&image_source))
    }

    async fn load_bytes_from_url(&self, url: &str) -> Result<Bytes, GalileoError> {
        let bytes = match self.load_from_cache(url) {
            Some(v) => v,
            None => {
                let loaded = self.load_from_web(url).await?;
                self.cache_controller.save_to_cache(url, &loaded);
                loaded
            }
        };

        Ok(bytes)
    }
}

impl NativePlatformService {
    fn load_from_cache(&self, url: &str) -> Option<Bytes> {
        let result = self.cache_controller.get_from_cache(url);
        if result.is_some() {
            info!("Loaded {url} from cache");
        }

        result
    }

    async fn load_from_web(&self, url: &str) -> Result<Bytes, GalileoError> {
        let response = self.http_client.get(url).send().await?;
        if !response.status().is_success() {
            info!(
                "Failed to load {url}: {}, {:?}",
                response.status(),
                response.text().await
            );
            return Err(GalileoError::IO);
        }

        Ok(response.bytes().await?)
    }
}
