use crate::error::GalileoError;
use crate::layer::data_provider::{DataDecoder, DataProvider, PersistentCacheController};
use crate::platform::PlatformService;
use bytes::Bytes;
use maybe_sync::{MaybeSend, MaybeSync};
use std::marker::PhantomData;

pub trait UrlSource<Key>: (Fn(&Key) -> String) + MaybeSend + MaybeSync {}
impl<Key, T: Fn(&Key) -> String> UrlSource<Key> for T where T: MaybeSend + MaybeSync {}

pub struct UrlDataProvider<Key, Decoder, Cache>
where
    Decoder: DataDecoder<Input = Bytes>,
    Cache: PersistentCacheController<Key, Bytes>,
{
    url_source: Box<dyn UrlSource<Key>>,
    decoder: Decoder,
    cache: Cache,
    _phantom_key: PhantomData<Key>,
}

impl<Key, Decoder, Cache> UrlDataProvider<Key, Decoder, Cache>
where
    Decoder: DataDecoder<Input = Bytes>,
    Cache: PersistentCacheController<Key, Bytes>,
{
    pub fn new(url_source: impl UrlSource<Key> + 'static, decoder: Decoder, cache: Cache) -> Self {
        Self {
            url_source: Box::new(url_source),
            decoder,
            cache,
            _phantom_key: Default::default(),
        }
    }

    async fn load_raw(&self, key: &Key) -> Result<Bytes, GalileoError> {
        if let Some(data) = self.cache.get(key) {
            return Ok(data);
        }

        let url = (self.url_source)(key);
        let data = crate::platform::PlatformServiceImpl::new()
            .load_bytes_from_url(&url)
            .await?;

        if let Err(error) = self.cache.insert(key, &data) {
            log::warn!("Failed to write persistent cache entry: {:?}", error);
        }

        Ok(data)
    }
}

impl<Key, Decoder, Cache> DataProvider<Key, Decoder::Output>
    for UrlDataProvider<Key, Decoder, Cache>
where
    Key: MaybeSend + MaybeSync,
    Decoder: DataDecoder<Input = Bytes> + MaybeSend + MaybeSync,
    Cache: PersistentCacheController<Key, Bytes> + MaybeSend + MaybeSync,
{
    async fn load(&self, key: &Key) -> Result<Decoder::Output, GalileoError> {
        let raw = self.load_raw(key).await?;
        self.decoder.decode(raw)
    }
}
