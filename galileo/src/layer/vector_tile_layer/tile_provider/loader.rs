//! Vector tile loader stuff.

use crate::error::GalileoError;
use crate::layer::data_provider::{PersistentCacheController, UrlSource};
use crate::platform::{PlatformService, PlatformServiceImpl};
use crate::tile_scheme::TileIndex;
use bytes::Bytes;
use galileo_mvt::MvtTile;
use maybe_sync::{MaybeSend, MaybeSync};

/// Error that can occur when trying to load a vector tile.
pub enum TileLoadError {
    /// Could not connect to the remote server.
    Network,
    /// Tile with the given index does not exist.
    DoesNotExist,
    /// Failed to decode vector tile from the binary data.
    Decoding,
}

/// Loader for vector tiles.
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait VectorTileLoader {
    /// Load tile with the given index.
    async fn load(&self, index: TileIndex) -> Result<MvtTile, TileLoadError>;
}

/// Load the tile from the Web.
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
    /// Create a new instance.
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
        if let Some(data) = self.cache.get(url) {
            log::trace!("Cache hit for url {url}");
            return Ok(data);
        }

        let bytes = self
            .platform_service
            .load_bytes_from_url(url)
            .await
            .map_err(|err| match err {
                GalileoError::NotFound => TileLoadError::DoesNotExist,
                _ => TileLoadError::Network,
            })?;

        log::info!("Loaded tile from url: {url}");

        if let Err(error) = self.cache.insert(url, &bytes) {
            log::warn!("Failed to write persistent cache entry: {:?}", error);
        }

        Ok(bytes)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
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
