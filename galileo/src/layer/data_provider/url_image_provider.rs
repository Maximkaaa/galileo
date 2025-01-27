#[cfg(target_arch = "wasm32")]
use std::future::Future;
use std::marker::PhantomData;

use bytes::Bytes;
use maybe_sync::{MaybeSend, MaybeSync};

use crate::decoded_image::DecodedImage;
use crate::error::GalileoError;
use crate::layer::data_provider::dummy::DummyCacheController;
use crate::layer::data_provider::{DataProvider, PersistentCacheController, UrlSource};
use crate::platform::{PlatformService, PlatformServiceImpl};

/// Loads an image from Internet and uses `Cache` persistent cache controller to save it locally.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub struct UrlImageProvider<Key, Cache = DummyCacheController> {
    url_source: Box<dyn UrlSource<Key>>,
    cache: Option<Cache>,
    platform_service: PlatformServiceImpl,
    offline_mode: bool,
    _phantom_key: PhantomData<Key>,
}

impl<Key> UrlImageProvider<Key, DummyCacheController> {
    /// Creates a new instance without persistent cache.
    pub fn new(url_source: impl UrlSource<Key> + 'static) -> Self {
        Self {
            url_source: Box::new(url_source),
            cache: None,
            platform_service: PlatformServiceImpl::new(),
            offline_mode: false,
            _phantom_key: Default::default(),
        }
    }
}

impl<Key, Cache> UrlImageProvider<Key, Cache> {
    /// Creates a new instance with persistent cache.
    pub fn new_cached(url_source: impl UrlSource<Key> + 'static, cache: Cache) -> Self {
        Self {
            url_source: Box::new(url_source),
            cache: Some(cache),
            platform_service: PlatformServiceImpl::new(),
            offline_mode: false,
            _phantom_key: Default::default(),
        }
    }

    /// If offline mode is enabled, the provider will not attempt to download data from Internet, and will only use
    /// its cache as the source of data.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_offline_mode(&mut self, enabled: bool) {
        if enabled && self.cache.is_none() {
            log::warn!("Offline mode for url image provider is enabled, but no persistent cache is configured.\
            No data will be available for this provider.")
        }

        self.offline_mode = enabled;
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn check_offline_mode(&self) -> Result<(), GalileoError> {
        if self.offline_mode {
            Err(GalileoError::NotFound)
        } else {
            Ok(())
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<Key, Cache> DataProvider<Key, DecodedImage, ()> for UrlImageProvider<Key, Cache>
where
    Key: MaybeSend + MaybeSync,
    Cache: PersistentCacheController<str, Bytes> + MaybeSend + MaybeSync,
{
    async fn load_raw(&self, key: &Key) -> Result<Bytes, GalileoError> {
        let url = (self.url_source)(key);

        if let Some(cache) = &self.cache {
            if let Some(data) = cache.get(&url) {
                return Ok(data);
            }
        }

        self.check_offline_mode()?;

        log::info!("Loading {url}");
        let data = self.platform_service.load_bytes_from_url(&url).await?;

        if let Some(cache) = &self.cache {
            if let Err(error) = cache.insert(&url, &data) {
                log::warn!("Failed to write persistent cache entry: {:?}", error);
            }
        }

        Ok(data)
    }

    fn decode(&self, bytes: Bytes, _context: ()) -> Result<DecodedImage, GalileoError> {
        DecodedImage::decode(&bytes)
    }
}

#[cfg(target_arch = "wasm32")]
impl<Key, Cache> DataProvider<Key, DecodedImage, ()> for UrlImageProvider<Key, Cache>
where
    Key: MaybeSend + MaybeSync,
    Cache: PersistentCacheController<str, Bytes> + MaybeSend + MaybeSync,
{
    fn load_raw(
        &self,
        _key: &Key,
    ) -> impl Future<Output = Result<Bytes, GalileoError>> + MaybeSend {
        std::future::ready(Err(GalileoError::Generic("not supported".into())))
    }

    fn decode(&self, _bytes: Bytes, _context: ()) -> Result<DecodedImage, GalileoError> {
        Err(GalileoError::Generic("not supported".into()))
    }

    async fn load(&self, key: &Key, _context: ()) -> Result<DecodedImage, GalileoError> {
        let url = (self.url_source)(key);
        self.platform_service.load_image_url(&url).await
    }
}
