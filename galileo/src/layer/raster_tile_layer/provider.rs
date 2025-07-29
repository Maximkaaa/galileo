use std::sync::Arc;

use bytes::Bytes;
use maybe_sync::{MaybeSend, MaybeSync};
use parking_lot::Mutex;
use quick_cache::sync::Cache;
use quick_cache::GuardResult;

use crate::decoded_image::DecodedImage;
use crate::error::GalileoError;
use crate::layer::data_provider::{PersistentCacheController, UrlSource};
use crate::layer::tiles::{RenderedState, TileProvider};
use crate::platform::PlatformService;
use crate::render::render_bundle::RenderBundle;
use crate::render::{Canvas, ImagePaint, PackedBundle};
use crate::tile_schema::TileIndex;
use crate::TileSchema;

/// Provider of tlies for a [`RusterTileLayer`](super::RasterTileLayer).
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait RasterTileLoader: MaybeSend + MaybeSync {
    /// Loads the tile with the given index.
    async fn load(&self, index: TileIndex) -> Result<DecodedImage, GalileoError>;
}

/// Raster tile loader that loads tiles one by one with REST HTTP GET requests.
///
/// This loader is able to load tiles from any protocol that use separate GET requests for each
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
/// use galileo::layer::raster_tile_layer::{RasterTileLoader, RestTileLoader};
/// use galileo::tile_schema::TileIndex;
///
/// let loader = RestTileLoader::new(
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
/// let tile = loader.load(TileIndex::new(3, 5, 3)).await.expect("failed to load tile");
/// # });
/// ```
pub struct RestTileLoader {
    url_source: Box<dyn UrlSource<TileIndex>>,
    cache: Option<Box<dyn PersistentCacheController<str, Bytes>>>,
    offline_mode: bool,
}

impl RestTileLoader {
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
impl RasterTileLoader for RestTileLoader {
    async fn load(&self, index: TileIndex) -> Result<DecodedImage, GalileoError> {
        let bytes = self.download_tile(index).await?;
        crate::platform::instance().decode_image(bytes).await
    }
}

#[derive(Clone)]
enum TileState {
    Loading,
    Loaded(Arc<DecodedImage>),
    Rendered(RenderedState),
    Error,
}

#[derive(Debug)]
pub(crate) struct RasterTileProvider {
    tiles: Mutex<Cache<TileIndex, TileState>>,
    tile_schema: TileSchema,
}

impl RasterTileProvider {
    pub(crate) fn new(tile_schema: TileSchema) -> Self {
        Self {
            tile_schema,
            tiles: Mutex::new(Cache::new(5000)),
        }
    }
}

impl RasterTileProvider {
    pub(crate) fn set_loading(&self, index: TileIndex) -> bool {
        match self.tiles.lock().get_value_or_guard(&index, None) {
            GuardResult::Value(_) => true,
            GuardResult::Guard(guard) => guard.insert(TileState::Loading).is_err(),
            GuardResult::Timeout => {
                log::error!("Raster tile provider is deadlocked");
                true
            }
        }
    }

    pub(crate) fn set_loaded(&self, index: TileIndex, image: DecodedImage) {
        self.tiles
            .lock()
            .insert(index, TileState::Loaded(Arc::new(image)));
    }

    pub(crate) fn set_error(&self, index: TileIndex) {
        self.tiles.lock().insert(index, TileState::Error);
    }

    pub(crate) fn pack_tiles(&self, indices: &[TileIndex], canvas: &dyn Canvas) {
        let tiles = self.tiles.lock();
        for index in indices {
            if let Some(TileState::Loaded(image)) = tiles.get(index) {
                let Some(tile_bbox) = self.tile_schema.tile_bbox(*index) else {
                    log::warn!("Failed to get bbox for tile {index:?}");
                    continue;
                };

                let mut bundle = RenderBundle::default();
                bundle.add_image(
                    image.clone(),
                    tile_bbox.into_quadrangle(),
                    ImagePaint { opacity: 255 },
                );
                let packed = canvas.pack_bundle(&bundle);
                tiles.insert(
                    *index,
                    TileState::Rendered(RenderedState {
                        bundle: packed.into(),
                        rendered_before: false, // Just rendered, not yet faded in
                    }),
                );
            }
        }
    }
}

impl TileProvider<()> for RasterTileProvider {
    fn get_tile(&self, index: TileIndex, _style_id: ()) -> Option<Arc<dyn PackedBundle>> {
        match self.tiles.lock().get(&index) {
            Some(TileState::Rendered(RenderedState { bundle, .. })) => Some(bundle),
            _ => None,
        }
    }

    fn get_rendered_state(&self, index: TileIndex, _style_id: ()) -> Option<RenderedState> {
        match self.tiles.lock().get(&index) {
            Some(TileState::Rendered(rendered)) => Some(rendered),
            _ => None,
        }
    }

    fn set_rendered_before(&self, index: TileIndex, _style_id: ()) {
        let tiles = self.tiles.lock();
        if let Some(TileState::Rendered(RenderedState {
            bundle,
            rendered_before: false,
        })) = tiles.get(&index)
        {
            tiles.insert(
                index,
                TileState::Rendered(RenderedState {
                    bundle,
                    rendered_before: true,
                }),
            );
        }
    }
}
