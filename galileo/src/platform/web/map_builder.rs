//! Map builder functions specific to Web target.

use std::sync::Arc;

use crate::galileo_map::MapBuilder as MapBuilderOld;
use crate::layer::data_provider::dummy::DummyCacheController;
use crate::layer::data_provider::UrlSource;
use crate::layer::raster_tile_layer::RestTileProvider;
use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::vector_tile_layer::tile_provider::loader::WebVtLoader;
use crate::layer::vector_tile_layer::tile_provider::VectorTileProvider;
use crate::layer::{RasterTileLayer, VectorTileLayer};
use crate::platform::web::vt_processor::WebWorkerVtProcessor;
use crate::platform::web::web_workers::WebWorkerService;
use crate::platform::{PlatformService, PlatformServiceImpl};
use crate::tile_scheme::TileIndex;
use crate::TileSchema;

impl MapBuilderOld {
    /// Creates a raster tile layer.
    pub fn create_raster_tile_layer(
        tile_source: impl UrlSource<TileIndex> + 'static,
        tile_scheme: TileSchema,
    ) -> RasterTileLayer {
        let tile_provider = RestTileProvider::new(tile_source, None, false);
        RasterTileLayer::new(tile_scheme, tile_provider, None)
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

    /// Create a new vector tile provider.
    pub fn create_vector_tile_provider(
        tile_source: impl UrlSource<TileIndex> + 'static,
        tile_schema: TileSchema,
    ) -> VectorTileProvider {
        let loader = WebVtLoader::new(
            PlatformServiceImpl::new(),
            DummyCacheController {},
            tile_source,
        );
        let ww_service = WebWorkerService::new(4);
        let processor = WebWorkerVtProcessor::new(tile_schema, ww_service);

        #[allow(clippy::arc_with_non_send_sync)]
        VectorTileProvider::new(Arc::new(loader), Arc::new(processor))
    }
}

pub(crate) async fn sleep(duration: i32) {
    let mut cb = |resolve: js_sys::Function, _reject: js_sys::Function| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, duration)
            .unwrap();
    };

    let p = js_sys::Promise::new(&mut cb);

    wasm_bindgen_futures::JsFuture::from(p).await.unwrap();
}
