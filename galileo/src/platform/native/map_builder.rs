//! Map builder functions that are present only on native platform.

use std::sync::Arc;

use galileo_types::geo::impls::GeoPoint2d;

use crate::layer::data_provider::{FileCacheController, UrlImageProvider, UrlSource};
use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::vector_tile_layer::tile_provider::loader::WebVtLoader;
use crate::layer::vector_tile_layer::tile_provider::VectorTileProvider;
use crate::layer::{RasterTileLayer, VectorTileLayer};
use crate::platform::native::vt_processor::ThreadVtProcessor;
use crate::platform::{PlatformService, PlatformServiceImpl};
use crate::render::render_bundle::tessellating::TessellatingRenderBundle;
use crate::render::render_bundle::{RenderBundle, RenderBundleType};
use crate::tile_scheme::TileIndex;
use crate::{MapBuilder, TileSchema};

impl MapBuilder {
    /// Creates a new instance.
    pub fn new() -> Self {
        Self {
            position: GeoPoint2d::default(),
            resolution: 156543.03392800014 / 16.0,
            view: None,
            layers: vec![],
            event_handlers: vec![],
            window: None,
            event_loop: None,
            size: None,
        }
    }

    /// Create a new raster tile layer.
    pub fn create_raster_tile_layer(
        tile_source: impl UrlSource<TileIndex> + 'static,
        tile_scheme: TileSchema,
    ) -> RasterTileLayer<UrlImageProvider<TileIndex, FileCacheController>> {
        #[cfg(not(target_os = "android"))]
        let cache_controller = FileCacheController::new(".tile_cache");

        #[cfg(target_os = "android")]
        let cache_controller =
            FileCacheController::new("/data/data/com.example.rastertilesandroid/.tile_cache");

        let tile_provider = UrlImageProvider::new_cached(tile_source, cache_controller);
        RasterTileLayer::new(tile_scheme, tile_provider, None)
    }

    /// Add a new raster layer to the layer list.
    pub fn with_raster_tiles(
        mut self,
        tile_source: impl UrlSource<TileIndex> + 'static,
        tile_scheme: TileSchema,
    ) -> Self {
        self.layers.push(Box::new(Self::create_raster_tile_layer(
            tile_source,
            tile_scheme,
        )));
        self
    }

    /// Create a new vector tile layer.
    pub fn create_vector_tile_layer(
        tile_source: impl UrlSource<TileIndex> + 'static,
        tile_schema: TileSchema,
        style: VectorTileStyle,
    ) -> VectorTileLayer {
        let tile_provider = Self::create_vector_tile_provider(tile_source, tile_schema.clone());
        VectorTileLayer::new(tile_provider, style, tile_schema)
    }

    /// Returns a vector tile provider.
    pub fn create_vector_tile_provider(
        tile_source: impl UrlSource<TileIndex> + 'static,
        tile_schema: TileSchema,
    ) -> VectorTileProvider {
        let loader = WebVtLoader::new(
            PlatformServiceImpl::new(),
            FileCacheController::new(".tile_cache"),
            tile_source,
        );
        let processor = ThreadVtProcessor::new(
            tile_schema,
            RenderBundle(RenderBundleType::Tessellating(
                TessellatingRenderBundle::new(),
            )),
        );

        VectorTileProvider::new(Arc::new(loader), Arc::new(processor))
    }
}
