use crate::error::GalileoError;
use crate::layer::data_provider::url_image_provider::dummy::DummyCacheController;
use crate::layer::data_provider::{DataProvider, PersistentCacheController};
use crate::platform::{PlatformService, PlatformServiceImpl};
use crate::primitives::DecodedImage;
use bytes::Bytes;
use maybe_sync::{MaybeSend, MaybeSync};
use std::marker::PhantomData;

pub trait UrlSource<Key>: (Fn(&Key) -> String) + MaybeSend + MaybeSync {}
impl<Key, T: Fn(&Key) -> String> UrlSource<Key> for T where T: MaybeSend + MaybeSync {}

pub struct UrlImageProvider<Key, Cache = DummyCacheController> {
    url_source: Box<dyn UrlSource<Key>>,
    cache: Option<Cache>,
    platform_service: PlatformServiceImpl,
    offline_mode: bool,
    _phantom_key: PhantomData<Key>,
}

impl<Key, Cache> UrlImageProvider<Key, Cache> {
    pub fn new(url_source: impl UrlSource<Key> + 'static, cache: Option<Cache>) -> Self {
        Self {
            url_source: Box::new(url_source),
            cache,
            platform_service: PlatformServiceImpl::new(),
            offline_mode: false,
            _phantom_key: Default::default(),
        }
    }

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

impl<Key, Cache> DataProvider<Key, DecodedImage> for UrlImageProvider<Key, Cache>
where
    Key: MaybeSend + MaybeSync,
    Cache: PersistentCacheController<str, Bytes> + MaybeSend + MaybeSync,
{
    async fn load(&self, key: &Key) -> Result<DecodedImage, GalileoError> {
        let url = (self.url_source)(key);

        if let Some(cache) = &self.cache {
            if let Some(data) = cache.get(&url) {
                return Ok(DecodedImage::new(&data)?);
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

        DecodedImage::new(&data)
    }
}

mod dummy {
    use crate::error::GalileoError;
    use crate::layer::data_provider::PersistentCacheController;
    use bytes::Bytes;

    #[allow(dead_code)]
    pub struct DummyCacheController {
        // Guarantees that the controller cannot be instantiated.
        private_field: u8,
    }

    impl<Key> PersistentCacheController<Key, Bytes> for DummyCacheController {
        fn get(&self, _key: &Key) -> Option<Bytes> {
            unreachable!()
        }

        fn insert(&self, _key: &Key, _data: &Bytes) -> Result<(), GalileoError> {
            unreachable!()
        }
    }
}
