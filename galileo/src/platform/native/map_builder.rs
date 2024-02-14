use crate::layer::data_provider::{
    FileCacheController, UrlDataProvider, UrlImageProvider, UrlSource,
};
use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::vector_tile_layer::tile_provider::ThreadedProvider;
use crate::layer::{RasterTileLayer, VectorTileLayer};
use crate::render::render_bundle::tessellating::TessellatingRenderBundle;
use crate::render::render_bundle::{RenderBundle, RenderBundleType};
use crate::tile_scheme::TileIndex;
use crate::{MapBuilder, TileSchema};
use galileo_types::geo::impls::GeoPoint2d;

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
        tile_scheme: TileSchema,
        style: VectorTileStyle,
    ) -> VectorTileLayer<
        ThreadedProvider<
            UrlDataProvider<
                TileIndex,
                crate::layer::vector_tile_layer::tile_provider::VtProcessor,
                FileCacheController,
            >,
        >,
    > {
        let tile_provider = ThreadedProvider::new(
            None,
            tile_scheme.clone(),
            UrlDataProvider::new_cached(
                tile_source,
                crate::layer::vector_tile_layer::tile_provider::VtProcessor {},
                FileCacheController::new(".tile_cache"),
            ),
            RenderBundle(RenderBundleType::Tessellating(
                TessellatingRenderBundle::new(),
            )),
        );

        VectorTileLayer::from_url(tile_provider, style, tile_scheme)
    }
}
