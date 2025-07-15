//! Thread vt processor.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use galileo_mvt::MvtTile;
use parking_lot::RwLock;

use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::vector_tile_layer::tile_provider::processor::{
    TileProcessingError, VectorTileProcessor,
};
use crate::layer::vector_tile_layer::tile_provider::{VtProcessor, VtStyleId};
use crate::render::render_bundle::RenderBundle;
use crate::tile_schema::TileIndex;
use crate::TileSchema;

/// Vector tile processor that uses a thread pool to run vector tile tessellation in parallel.
pub struct ThreadVtProcessor {
    tile_schema: TileSchema,
    styles: RwLock<HashMap<VtStyleId, Arc<VectorTileStyle>>>,
}

impl ThreadVtProcessor {
    /// Create a new instance of the processor.
    pub fn new(tile_schema: TileSchema) -> Self {
        Self {
            tile_schema,
            styles: Default::default(),
        }
    }
}

#[async_trait::async_trait]
impl VectorTileProcessor for ThreadVtProcessor {
    fn has_style(&self, style_id: VtStyleId) -> bool {
        self.styles.read().contains_key(&style_id)
    }

    fn get_style(&self, style_id: VtStyleId) -> Option<Arc<VectorTileStyle>> {
        self.styles.read().get(&style_id).cloned()
    }

    fn add_style(&self, style_id: VtStyleId, style: VectorTileStyle) {
        self.styles.write().insert(style_id, Arc::new(style));
    }

    fn drop_style(&self, style_id: VtStyleId) {
        self.styles.write().remove(&style_id);
    }

    async fn process_tile(
        &self,
        tile: Arc<MvtTile>,
        index: TileIndex,
        style_id: VtStyleId,
        dpi_scale_factor: f32,
    ) -> Result<RenderBundle, TileProcessingError> {
        // todo: remove clone here
        let Some(style) = self.styles.read().get(&style_id).cloned() else {
            return Err(TileProcessingError::InvalidStyle);
        };

        let mut bundle = RenderBundle::default();
        let tile_schema = self.tile_schema.clone();

        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        tokio::task::spawn_blocking(move || {
            log::debug!(
                "Added worker: {}",
                COUNTER.fetch_add(1, Ordering::Relaxed) + 1
            );
            let result = match VtProcessor::prepare(
                &tile,
                &mut bundle,
                index,
                &style,
                &tile_schema,
                dpi_scale_factor,
            ) {
                Ok(()) => Ok(bundle),
                Err(_) => Err(TileProcessingError::Rendering),
            };
            log::debug!(
                "Finished worker: {}",
                COUNTER.fetch_sub(1, Ordering::Relaxed) - 1
            );
            result
        })
        .await
        .map_err(|_| TileProcessingError::Rendering)?
    }
}
