//! Thread vt processor.

use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::vector_tile_layer::tile_provider::processor::{
    TileProcessingError, VectorTileProcessor,
};
use crate::layer::vector_tile_layer::tile_provider::{VtProcessor, VtStyleId};
use crate::render::render_bundle::RenderBundle;
use crate::tile_scheme::TileIndex;
use crate::TileSchema;
use galileo_mvt::MvtTile;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};

/// Vector tile processor that uses a thread pool to run vector tile tessellation in parallel.
pub struct ThreadVtProcessor {
    tile_schema: TileSchema,
    empty_bundle: RenderBundle,
    styles: RwLock<HashMap<VtStyleId, Arc<VectorTileStyle>>>,
}

impl ThreadVtProcessor {
    /// Create a new instance of the processor.
    pub fn new(tile_schema: TileSchema, empty_bundle: RenderBundle) -> Self {
        Self {
            tile_schema,
            empty_bundle,
            styles: Default::default(),
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl VectorTileProcessor for ThreadVtProcessor {
    fn has_style(&self, style_id: VtStyleId) -> bool {
        self.styles
            .read()
            .expect("lock is poisoned")
            .contains_key(&style_id)
    }

    fn get_style(&self, style_id: VtStyleId) -> Option<Arc<VectorTileStyle>> {
        self.styles
            .read()
            .expect("lock is poisoned")
            .get(&style_id)
            .cloned()
    }

    async fn add_style(&self, style_id: VtStyleId, style: VectorTileStyle) {
        self.styles
            .write()
            .expect("lock is poisoned")
            .insert(style_id, Arc::new(style));
    }

    async fn drop_style(&self, style_id: VtStyleId) {
        self.styles
            .write()
            .expect("lock is poisoned")
            .remove(&style_id);
    }

    async fn process_tile(
        &self,
        tile: Arc<MvtTile>,
        index: TileIndex,
        style_id: VtStyleId,
    ) -> Result<RenderBundle, TileProcessingError> {
        // todo: remove clone here
        let Some(style) = self
            .styles
            .read()
            .expect("lock is poisoned")
            .get(&style_id)
            .cloned()
        else {
            return Err(TileProcessingError::InvalidStyle);
        };

        let mut bundle = self.empty_bundle.clone();
        let tile_schema = self.tile_schema.clone();

        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        tokio::task::spawn_blocking(move || {
            log::debug!(
                "Added worker: {}",
                COUNTER.fetch_add(1, Ordering::Relaxed) + 1
            );
            let result = match VtProcessor::prepare(&tile, &mut bundle, index, &style, &tile_schema)
            {
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
