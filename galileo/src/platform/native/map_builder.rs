//! Map builder functions that are present only on native platform.

use std::sync::Arc;

use crate::layer::data_provider::{FileCacheController, UrlSource};
use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::vector_tile_layer::tile_provider::loader::WebVtLoader;
use crate::layer::vector_tile_layer::tile_provider::VectorTileProvider;
use crate::layer::VectorTileLayer;
use crate::platform::native::vt_processor::ThreadVtProcessor;
use crate::platform::{PlatformService, PlatformServiceImpl};
use crate::render::render_bundle::tessellating::TessellatingRenderBundle;
use crate::render::render_bundle::{RenderBundle, RenderBundleType};
use crate::tile_schema::TileIndex;
use crate::{MapBuilderOld, TileSchema};

impl MapBuilderOld {
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
            FileCacheController::new(".tile_cache").expect("failed to create cache controller"),
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
