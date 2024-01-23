use crate::error::GalileoError;
use crate::layer::data_provider::{DataProcessor, DataProvider, PersistentCacheController};
use crate::platform::{PlatformService, PlatformServiceImpl};
use bytes::Bytes;
use maybe_sync::{MaybeSend, MaybeSync};
use std::marker::PhantomData;

pub trait UrlSource<Key: ?Sized>: (Fn(&Key) -> String) + MaybeSend + MaybeSync {}
impl<Key: ?Sized, T: Fn(&Key) -> String> UrlSource<Key> for T where T: MaybeSend + MaybeSync {}

pub struct UrlDataProvider<Key, Decoder, Cache>
where
    Key: ?Sized,
    Decoder: DataProcessor<Input = Bytes>,
    Cache: PersistentCacheController<str, Bytes>,
{
    url_source: Box<dyn UrlSource<Key>>,
    decoder: Decoder,
    cache: Cache,
    platform_service: PlatformServiceImpl,
    _phantom_key: PhantomData<Key>,
}

impl<Key, Decoder, Cache> UrlDataProvider<Key, Decoder, Cache>
where
    Key: ?Sized,
    Decoder: DataProcessor<Input = Bytes>,
    Cache: PersistentCacheController<str, Bytes>,
{
    pub fn new(url_source: impl UrlSource<Key> + 'static, decoder: Decoder, cache: Cache) -> Self {
        Self {
            url_source: Box::new(url_source),
            decoder,
            cache,
            platform_service: PlatformServiceImpl::new(),
            _phantom_key: Default::default(),
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
        if let Some(data) = self.cache.get(&url) {
            return Ok(data);
        }

        let data = self.platform_service.load_bytes_from_url(&url).await?;

        if let Err(error) = self.cache.insert(&url, &data) {
            log::warn!("Failed to write persistent cache entry: {:?}", error);
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
