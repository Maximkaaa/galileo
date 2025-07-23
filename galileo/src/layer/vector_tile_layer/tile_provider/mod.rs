//! Vector tile layer tile providers

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use galileo_mvt::MvtTile;
use loader::VectorTileLoader;
use parking_lot::RwLock;
use processor::VectorTileProcessor;

use crate::layer::tiles::TileProvider;
use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::messenger::Messenger;
use crate::render::{Canvas, PackedBundle};
use crate::tile_schema::TileIndex;

pub mod loader;
pub mod processor;
mod tile_store;
mod vt_processor;

pub use vt_processor::{VectorTileDecodeContext, VtProcessor};

use crate::layer::vector_tile_layer::tile_provider::tile_store::{
    MvtTileState, PreparedTileState, TileStore,
};

/// Identifier of a vector tile style.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VtStyleId(u32);

impl VtStyleId {
    fn next_id() -> Self {
        static ID: AtomicU32 = AtomicU32::new(0);
        Self(ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// Provider of vector tiles for a vector tile layer.
pub struct VectorTileProvider {
    tiles: Arc<RwLock<TileStore>>,
    loader: Arc<dyn VectorTileLoader>,
    processor: Arc<dyn VectorTileProcessor>,
    messenger: Option<Arc<dyn Messenger>>,
}

impl Clone for VectorTileProvider {
    fn clone(&self) -> Self {
        Self {
            tiles: self.tiles.clone(),
            loader: self.loader.clone(),
            processor: self.processor.clone(),
            messenger: self.messenger.clone(),
        }
    }
}

impl TileProvider<VtStyleId> for VectorTileProvider {
    fn get_tile(&self, index: TileIndex, style_id: VtStyleId) -> Option<Arc<dyn PackedBundle>> {
        VectorTileProvider::get_tile(self, index, style_id)
    }
}

impl VectorTileProvider {
    /// Create a new instance of the provider.
    pub fn new(loader: Arc<dyn VectorTileLoader>, processor: Arc<dyn VectorTileProcessor>) -> Self {
        Self {
            tiles: Arc::default(),
            loader,
            processor,
            messenger: None,
        }
    }

    /// Return the style with the given id.
    pub fn get_style(&self, style_id: VtStyleId) -> Option<Arc<VectorTileStyle>> {
        self.processor.get_style(style_id)
    }

    /// Register a new style in the provider.
    pub fn add_style(&mut self, style: VectorTileStyle) -> VtStyleId {
        let id = VtStyleId::next_id();
        self.processor.add_style(id, style);

        id
    }

    /// Removes the style from the list of registered styles.
    pub fn drop_style(&mut self, style_id: VtStyleId) {
        self.processor.drop_style(style_id);
    }

    /// Load and pre-render the tile with given index using given style.
    ///
    /// A style with given id must first be registered in the provider.
    pub fn load_tile(&self, index: TileIndex, style_id: VtStyleId) {
        if !self.processor.has_style(style_id) {
            log::warn!("Requested tile loading with non-existing style");
            return;
        }

        let tile_store = self.tiles.clone();
        if tile_store.read().contains(index, style_id) {
            return;
        }

        log::debug!("Loading vector tile {index:?}");

        let processor = self.processor.clone();
        let data_provider = self.loader.clone();
        let messenger = self.messenger.clone();

        crate::async_runtime::spawn(async move {
            let cell = {
                let mut store = tile_store.write();
                if store.contains(index, style_id) {
                    return;
                }

                store.start_loading_tile(index, style_id)
            };

            let tile_state = cell
                .get_or_init(|| async { Self::download(index, data_provider).await })
                .await;

            log::debug!("Tile {index:?} is loaded. Preparing.");

            let tile_state = Self::prepare_tile(tile_state, index, style_id, processor).await;

            log::debug!("tile {index:?} is prepared.");

            tile_store
                .write()
                .store_tile(index, style_id, cell, tile_state);

            if let Some(messenger) = messenger {
                messenger.request_redraw();
            }
        });
    }

    /// Move the pre-renderred tile data into GPU memory.
    ///
    /// If any of the tiles with the given indices was not pre-renderred with the given style id,
    /// it is just skipped.
    pub fn pack_tiles(&self, indices: &[TileIndex], style_id: VtStyleId, canvas: &dyn Canvas) {
        let mut store = self.tiles.write();
        for index in indices {
            if let Some((tile, mvt_tile)) = store.get_prepared(*index, style_id) {
                let bundle = (*tile).clone();
                let packed = canvas.pack_bundle(&bundle);
                store.store_tile(
                    *index,
                    style_id,
                    mvt_tile,
                    PreparedTileState::Packed(packed.into()),
                );
            }
        }
    }

    /// Return render bundle for given tile.
    ///
    /// The tile must be packed before calling this method.
    pub fn get_tile(&self, index: TileIndex, style_id: VtStyleId) -> Option<Arc<dyn PackedBundle>> {
        self.tiles.read().get_packed(index, style_id)
    }

    /// Returns raw tile data for the given index.
    pub fn get_mvt_tile(&self, index: TileIndex) -> Option<Arc<MvtTile>> {
        self.tiles.read().get_mvt_tile(index)
    }

    /// Set messenger to use to notify about tile updates.
    pub fn set_messenger(&mut self, messenger: Box<dyn Messenger>) {
        self.messenger = Some(messenger.into());
    }

    /// Notifies the messenger about a change to be processed by the map.
    // TODO: This method should not be here. This requires some refactoring.
    pub fn request_redraw(&self) {
        if let Some(messenger) = &self.messenger {
            messenger.request_redraw();
        }
    }

    async fn download(tile_index: TileIndex, loader: Arc<dyn VectorTileLoader>) -> MvtTileState {
        match loader.load(tile_index).await {
            Ok(mvt_tile) => MvtTileState::Loaded(Arc::new(mvt_tile)),
            Err(_) => MvtTileState::Error(),
        }
    }

    async fn prepare_tile(
        mvt_tile_state: &MvtTileState,
        index: TileIndex,
        style_id: VtStyleId,
        processor: Arc<dyn VectorTileProcessor>,
    ) -> PreparedTileState {
        match mvt_tile_state {
            MvtTileState::Loaded(mvt_tile) => {
                match processor
                    .process_tile(mvt_tile.clone(), index, style_id)
                    .await
                {
                    Ok(render_bundle) => PreparedTileState::Loaded(Arc::new(render_bundle)),
                    Err(_) => PreparedTileState::Error,
                }
            }
            MvtTileState::Error() => PreparedTileState::Error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        let id1 = VtStyleId::next_id();
        let id2 = VtStyleId::next_id();
        let id3 = VtStyleId::next_id();

        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
    }
}
