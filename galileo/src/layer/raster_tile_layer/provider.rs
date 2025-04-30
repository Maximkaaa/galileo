use bytes::Bytes;
use maybe_sync::{MaybeSend, MaybeSync};

use crate::decoded_image::DecodedImage;
use crate::error::GalileoError;
use crate::layer::data_provider::{PersistentCacheController, UrlSource};
use crate::platform::PlatformService;
use crate::tile_schema::TileIndex;

/// Provider of tlies for a [`RusterTileLayer`](super::RasterTileLayer).
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait RasterTileProvider: MaybeSend + MaybeSync {
    /// Loads the tile with the given index.
    async fn load(&self, index: TileIndex) -> Result<DecodedImage, GalileoError>;
}

/// Raster tile provider that loads tiles one by one with REST HTTP GET requests.
///
/// This provider is able to load tiles from any protocol that use separate GET requests for each
/// tiles:
/// * OSM tile protocol
/// * OSG Tile Map Service (TMS)
/// * ArcGis TileService
/// * etc.
///
/// If constructed with a [`PersistentCacheController`] it will cache the loaded tiles and only
/// request new tiles from the source url if they are not in the cache.
///
/// If configured to use offline mode, it will only use tiles from the cache without attempting to
/// load them from the source. Nevertheless, even in this case url source must be correct to
/// identify the correct files to retrieve from the cache.
///
/// # Example
///
/// ```no_run
/// use galileo::layer::raster_tile_layer::{RasterTileProvider, RestTileProvider};
/// use galileo::tile_schema::TileIndex;
///
/// let provider = RestTileProvider::new(
///     |index| {
///         format!(
///             "https://tile.openstreetmap.org/{}/{}/{}.png",
///             index.z, index.x, index.y
///         )
///     },
///     None,
///     false
///     );
///
/// # tokio_test::block_on(async {
/// let tile = provider.load(TileIndex::new(3, 5, 3)).await.expect("failed to load tile");
/// # });
/// ```
pub struct RestTileProvider {
    url_source: Box<dyn UrlSource<TileIndex>>,
    cache: Option<Box<dyn PersistentCacheController<str, Bytes>>>,
    offline_mode: bool,
}

impl RestTileProvider {
    /// Creates a new instance of the provider.
    pub fn new(
        url_source: impl UrlSource<TileIndex> + 'static,
        cache: Option<Box<dyn PersistentCacheController<str, Bytes>>>,
        offline_mode: bool,
    ) -> Self {
        Self {
            url_source: Box::new(url_source),
            cache,
            offline_mode,
        }
    }

    async fn download_tile(&self, index: TileIndex) -> Result<Bytes, GalileoError> {
        let url = (self.url_source)(&index);

        if let Some(cache) = &self.cache {
            if let Some(data) = cache.get(&url) {
                return Ok(data);
            }
        }

        if self.offline_mode {
            return Err(GalileoError::NotFound);
        }

        log::info!("Loading {url}");
        let data = crate::platform::instance()
            .load_bytes_from_url(&url)
            .await?;

        if let Some(cache) = &self.cache {
            if let Err(error) = cache.insert(&url, &data) {
                log::warn!("Failed to write persistent cache entry: {error:?}");
            }
        }

        Ok(data)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl RasterTileProvider for RestTileProvider {
    async fn load(&self, index: TileIndex) -> Result<DecodedImage, GalileoError> {
        let bytes = self.download_tile(index).await?;
        crate::platform::instance().decode_image(bytes).await
    }
}
