use crate::error::GalileoError;
use crate::layer::data_provider::dummy::DummyCacheController;
use crate::layer::data_provider::{
    DataProcessor, DataProvider, PersistentCacheController, UrlSource,
};
use crate::platform::{PlatformService, PlatformServiceImpl};
use bytes::Bytes;
use maybe_sync::{MaybeSend, MaybeSync};
use std::marker::PhantomData;

/// Loads data from Internet and uses `Cache` persistent cache to save data locally.
pub struct UrlDataProvider<Key, Decoder, Cache = DummyCacheController>
where
    Key: ?Sized,
    Decoder: DataProcessor<Input = Bytes>,
    Cache: PersistentCacheController<str, Bytes>,
{
    url_source: Box<dyn UrlSource<Key>>,
    decoder: Decoder,
    cache: Option<Cache>,
    offline_mode: bool,
    platform_service: PlatformServiceImpl,
    _phantom_key: PhantomData<Key>,
}

impl<Key, Decoder> UrlDataProvider<Key, Decoder, DummyCacheController>
where
    Key: ?Sized,
    Decoder: DataProcessor<Input = Bytes>,
{
    /// Creates a new instance without persistent cache.
    pub fn new(url_source: impl UrlSource<Key> + 'static, decoder: Decoder) -> Self {
        Self {
            url_source: Box::new(url_source),
            decoder,
            cache: None,
            offline_mode: false,
            platform_service: PlatformServiceImpl::new(),
            _phantom_key: Default::default(),
        }
    }
}

impl<Key, Decoder, Cache> UrlDataProvider<Key, Decoder, Cache>
where
    Key: ?Sized + MaybeSend + MaybeSync,
    Decoder: DataProcessor<Input = Bytes> + MaybeSend + MaybeSync,
    Decoder::Context: MaybeSend + MaybeSync,
    Cache: PersistentCacheController<str, Bytes> + MaybeSend + MaybeSync,
{
    /// Creates a new instance with persistent cache.
    pub fn new_cached(
        url_source: impl UrlSource<Key> + 'static,
        decoder: Decoder,
        cache: Cache,
    ) -> Self {
        Self {
            url_source: Box::new(url_source),
            decoder,
            cache: Some(cache),
            offline_mode: false,
            platform_service: PlatformServiceImpl::new(),
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

    fn check_offline_mode(&self) -> Result<(), GalileoError> {
        if self.offline_mode {
            Err(GalileoError::NotFound)
        } else {
            Ok(())
        }
    }
}

impl<Key, Decoder, Cache> DataProvider<Key, Decoder::Output, Decoder::Context>
    for UrlDataProvider<Key, Decoder, Cache>
where
    Key: ?Sized + MaybeSend + MaybeSync,
    Decoder: DataProcessor<Input = Bytes> + MaybeSend + MaybeSync,
    Decoder::Context: MaybeSend + MaybeSync,
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

        let data = self.platform_service.load_bytes_from_url(&url).await?;

        if let Some(cache) = &self.cache {
            if let Err(error) = cache.insert(&url, &data) {
                log::warn!("Failed to write persistent cache entry: {:?}", error);
            }
        }

        Ok(data)
    }

    fn decode(
        &self,
        raw: Bytes,
        context: Decoder::Context,
    ) -> Result<Decoder::Output, GalileoError> {
        self.decoder.process(raw, context)
    }
}
