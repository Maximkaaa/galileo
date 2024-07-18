use crate::error::GalileoError;
use crate::layer::data_provider::{PersistentCacheController, UrlSource};
use crate::platform::{PlatformService, PlatformServiceImpl};
use crate::tile_scheme::TileIndex;
use bytes::Bytes;
use galileo_mvt::MvtTile;
use maybe_sync::{MaybeSend, MaybeSync};
use reqwest::Client;

pub(super) enum TileLoadError {
    Network,
    DoesNotExist,
    Decoding,
}

#[async_trait::async_trait]
pub trait VectorTileLoader {
    async fn load(&self, index: TileIndex) -> Result<MvtTile, TileLoadError>;
}

pub struct WebVtLoader<Cache>
where
    Cache: PersistentCacheController<str, Bytes> + MaybeSend + MaybeSync,
{
    platform_service: PlatformServiceImpl,
    cache: Cache,
    url_source: Box<dyn UrlSource<TileIndex>>,
}

impl<Cache> WebVtLoader<Cache>
where
    Cache: PersistentCacheController<str, Bytes> + MaybeSend + MaybeSync,
{
    pub fn new(
        platform_service: PlatformServiceImpl,
        cache: Cache,
        url_source: impl UrlSource<TileIndex> + 'static,
    ) -> Self {
        Self {
            platform_service,
            cache,
            url_source: Box::new(url_source),
        }
    }

    async fn load_raw(&self, url: &str) -> Result<Bytes, TileLoadError> {
        if let Some(data) = self.cache.get(&url) {
            log::trace!("Cache hit for url {url}");
            return Ok(data);
        }

        let bytes = self
            .platform_service
            .load_bytes_from_url(&url)
            .await
            .map_err(|err| match err {
                GalileoError::NotFound => TileLoadError::DoesNotExist,
                _ => TileLoadError::Network,
            })?;

        if let Err(error) = self.cache.insert(&url, &bytes) {
            log::warn!("Failed to write persistent cache entry: {:?}", error);
        }

        Ok(bytes)
    }
}

#[async_trait::async_trait]
impl<Cache> VectorTileLoader for WebVtLoader<Cache>
where
    Cache: PersistentCacheController<str, Bytes> + MaybeSend + MaybeSync,
{
    async fn load(&self, index: TileIndex) -> Result<MvtTile, TileLoadError> {
        let url = (self.url_source)(&index);

        log::trace!("Loading tile {index:?} from url {url}");
        let bytes = self.load_raw(&url).await?;

        log::trace!("Tile {index:?} loaded. Byte size: {}", bytes.len());

        let mvt = MvtTile::decode(bytes, false).map_err(|_| TileLoadError::Decoding)?;

        log::trace!("Tile {index:?} successfully decoded");

        Ok(mvt)
    }
}
