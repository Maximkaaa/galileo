//! Vector tile processor implementation for Web

use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::vector_tile_layer::tile_provider::processor::{
    TileProcessingError, VectorTileProcessor,
};
use crate::layer::vector_tile_layer::tile_provider::VtStyleId;
use crate::platform::web::web_workers::WebWorkerService;
use crate::render::render_bundle::RenderBundle;
use crate::tile_scheme::TileIndex;
use crate::TileSchema;
use async_trait::async_trait;
use galileo_mvt::MvtTile;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

/// Vector tile processor that uses Web Workers to prepare tiles for rendering.
pub struct WebWorkerVtProcessor {
    tile_schema: TileSchema,
    styles: RefCell<HashMap<VtStyleId, Arc<VectorTileStyle>>>,
    ww_service: WebWorkerService,
}

impl WebWorkerVtProcessor {
    /// Create new instance.
    pub fn new(tile_schema: TileSchema, ww_service: WebWorkerService) -> Self {
        Self {
            tile_schema,
            styles: RefCell::new(HashMap::new()),
            ww_service,
        }
    }
}

#[async_trait(?Send)]
impl VectorTileProcessor for WebWorkerVtProcessor {
    fn has_style(&self, style_id: VtStyleId) -> bool {
        self.styles.borrow().contains_key(&style_id)
    }

    fn get_style(&self, style_id: VtStyleId) -> Option<Arc<VectorTileStyle>> {
        self.styles.borrow().get(&style_id).cloned()
    }

    async fn add_style(&self, style_id: VtStyleId, style: VectorTileStyle) {
        self.styles.borrow_mut().insert(style_id, Arc::new(style));
    }

    async fn drop_style(&self, style_id: VtStyleId) {
        self.styles.borrow_mut().remove(&style_id);
    }

    async fn process_tile(
        &self,
        tile: Arc<MvtTile>,
        index: TileIndex,
        style_id: VtStyleId,
    ) -> Result<RenderBundle, TileProcessingError> {
        let Some(style) = self.get_style(style_id) else {
            return Err(TileProcessingError::InvalidStyle);
        };

        self.ww_service
            .process_vt_tile(tile, index, style, self.tile_schema.clone())
            .await
    }
}
