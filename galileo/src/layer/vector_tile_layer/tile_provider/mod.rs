//! Vector tile layer tile providers

use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::vector_tile_layer::vector_tile::VectorTile;
use crate::messenger::Messenger;
use crate::render::render_bundle::RenderBundle;
use crate::render::{Canvas, PackedBundle};
use crate::tile_scheme::TileIndex;
use galileo_mvt::MvtTile;
use loader::VectorTileLoader;
use maybe_sync::{AtomicU32, MaybeSend, MaybeSync};
use processor::VectorTileProcessor;
use quick_cache::unsync::Cache;
use std::sync::atomic::Ordering;
use std::sync::{Arc, MutexGuard, RwLock};

#[cfg(target_arch = "wasm32")]
mod web_worker_provider;
#[cfg(target_arch = "wasm32")]
pub use web_worker_provider::WebWorkerVectorTileProvider;

#[cfg(not(target_arch = "wasm32"))]
mod threaded_provider;
#[cfg(not(target_arch = "wasm32"))]
pub use threaded_provider::ThreadedProvider;

pub mod loader;
pub mod processor;
mod tile_store;
mod vt_processor;

use crate::layer::vector_tile_layer::tile_provider::tile_store::{
    MvtTileState, PreparedTileState, TileStore,
};
pub use vt_processor::{VectorTileDecodeContext, VtProcessor};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VtStyleId(u32);

impl VtStyleId {
    fn next_id() -> Self {
        static ID: AtomicU32 = AtomicU32::new(0);
        Self(ID.fetch_add(1, Ordering::Relaxed))
    }
}

pub struct VectorTileProvider<Loader, Processor>
where
    Loader: VectorTileLoader + Send + Sync + 'static,
    Processor: VectorTileProcessor + Send + Sync + 'static,
{
    tiles: Arc<RwLock<TileStore>>,
    loader: Arc<Loader>,
    processor: Arc<Processor>,
    messenger: Option<Arc<dyn Messenger>>,
}

impl<Loader, Processor> Clone for VectorTileProvider<Loader, Processor>
where
    Loader: VectorTileLoader + Send + Sync + 'static,
    Processor: VectorTileProcessor + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            tiles: self.tiles.clone(),
            loader: self.loader.clone(),
            processor: self.processor.clone(),
            messenger: self.messenger.clone(),
        }
    }
}

impl<Loader, Processor> VectorTileProvider<Loader, Processor>
where
    Loader: VectorTileLoader + Send + Sync + 'static,
    Processor: VectorTileProcessor + Send + Sync + 'static,
{
    pub fn new(loader: Arc<Loader>, processor: Arc<Processor>) -> Self {
        Self {
            tiles: Arc::default(),
            loader,
            processor,
            messenger: None,
        }
    }

    pub fn get_style(&self, style_id: VtStyleId) -> Option<Arc<VectorTileStyle>> {
        self.processor.get_style(style_id)
    }

    pub async fn add_style(&mut self, style: VectorTileStyle) -> VtStyleId {
        let id = VtStyleId::next_id();
        self.processor.add_style(id, style).await;

        id
    }

    pub async fn drop_style(&mut self, style_id: VtStyleId) {
        self.processor.drop_style(style_id).await;
    }

    pub fn load_tile(&self, index: TileIndex, style_id: VtStyleId) {
        if !self.processor.has_style(style_id) {
            log::warn!("Requested tile loading with non-existing style");
            return;
        }

        let tile_store = self.tiles.clone();
        if tile_store
            .read()
            .expect("lock is poisoned")
            .contains(index, style_id)
        {
            return;
        }

        let processor = self.processor.clone();
        let data_provider = self.loader.clone();
        let messenger = self.messenger.clone();

        crate::async_runtime::spawn(async move {
            let cell = {
                let mut store = tile_store.write().expect("lock is poisoned");
                if store.contains(index, style_id) {
                    return;
                }

                store.start_loading_tile(index, style_id)
            };

            let tile_state = cell
                .get_or_init(|| async { Self::download(index, data_provider).await })
                .await;

            let tile_state = Self::prepare_tile(tile_state, index, style_id, processor).await;
            tile_store
                .write()
                .expect("lock is poisoned")
                .store_tile(index, style_id, cell, tile_state);

            if let Some(messenger) = messenger {
                messenger.request_redraw();
            }
        });
    }

    pub fn pack_tiles(&self, indices: &[TileIndex], style_id: VtStyleId, canvas: &dyn Canvas) {
        let mut store = self.tiles.write().expect("lock is poisoned");
        for index in indices {
            if let Some((tile, mvt_tile)) = store.get_prepared(*index, style_id) {
                let packed = canvas.pack_bundle(&tile);
                store.store_tile(
                    *index,
                    style_id,
                    mvt_tile,
                    PreparedTileState::Packed(packed.into()),
                );
            }
        }
    }

    pub fn get_tile(&self, index: TileIndex, style_id: VtStyleId) -> Option<Arc<dyn PackedBundle>> {
        self.tiles
            .read()
            .expect("lock is poisoned")
            .get_packed(index, style_id)
    }

    pub fn get_mvt_tile(&self, index: TileIndex) -> Option<Arc<MvtTile>> {
        self.tiles
            .read()
            .expect("lock is poisoned")
            .get_mvt_tile(index)
    }

    pub fn set_messenger(&mut self, messenger: Box<dyn Messenger>) {
        self.messenger = Some(messenger.into());
    }

    async fn download(tile_index: TileIndex, loader: Arc<Loader>) -> MvtTileState {
        match loader.load(tile_index).await {
            Ok(mvt_tile) => MvtTileState::Loaded(Arc::new(mvt_tile)),
            Err(_) => MvtTileState::Error(),
        }
    }

    async fn prepare_tile(
        mvt_tile_state: &MvtTileState,
        index: TileIndex,
        style_id: VtStyleId,
        processor: Arc<Processor>,
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

/// Vector tile provider.
pub trait VectorTileProviderT: MaybeSend + MaybeSync {
    /// Load a tile with the given index, and prerender it with the given style.
    fn load_tile(&self, index: TileIndex, style: &VectorTileStyle);
    /// Update the style of the loaded tiles.
    fn update_style(&self);
    /// Returns a lock of the tile store.
    fn read(&self) -> LockedTileStore;
    /// Set a messenger to notify the application when a new tile is loaded.
    fn set_messenger(&self, messenger: Box<dyn Messenger>);
}

/// Lock of the tile store. Only one lock can be held at a time.
pub struct LockedTileStore<'a> {
    guard: MutexGuard<'a, Cache<TileIndex, TileState>>,
}

impl<'a> LockedTileStore<'a> {
    /// Returns a raw MVT tile by the index.
    ///
    /// Returns `None` if the tile with the given index is not in the store.
    pub fn get_mvt_tile(&'a self, index: TileIndex) -> Option<&'a MvtTile> {
        self.guard.get(&index).and_then(|v| match v {
            TileState::Loaded(tile) => Some(&tile.mvt_tile),
            TileState::Packed(tile) | TileState::Updating(tile) | TileState::Outdated(tile) => {
                Some(&tile.mvt_tile)
            }
            _ => None,
        })
    }

    /// Packs the tile with the given index using the `canvas`.
    ///
    /// If tile does not exist, does nothing.
    pub fn pack(&mut self, index: TileIndex, canvas: &dyn Canvas) {
        if self.needs_packing(&index) {
            let tile_state = self.guard.remove(&index);
            match tile_state {
                Some((_, TileState::Loaded(tile))) => {
                    let UnpackedVectorTile { bundle, mvt_tile } = *tile;
                    let packed = canvas.pack_bundle(&bundle);
                    self.guard.insert(
                        index,
                        TileState::Packed(VectorTile {
                            mvt_tile,
                            bundle: packed,
                        }),
                    );
                }
                _ => {
                    log::error!("Tried to pack a tile in not packable state");
                }
            }
        }
    }

    /// Returns a tile with the given index, if the tile was loaded and packed.
    pub fn get_tile(&'a self, index: TileIndex) -> Option<&'a VectorTile> {
        self.guard.get(&index).and_then(|v| match v {
            TileState::Packed(tile) | TileState::Outdated(tile) | TileState::Updating(tile) => {
                Some(tile)
            }
            _ => None,
        })
    }

    fn needs_packing(&self, index: &TileIndex) -> bool {
        self.guard
            .get(index)
            .is_some_and(|tile_state| matches!(tile_state, TileState::Loaded(_)))
    }
}

struct UnpackedVectorTile {
    mvt_tile: MvtTile,
    bundle: RenderBundle,
}

enum TileState {
    Loading,
    Loaded(Box<UnpackedVectorTile>),
    Outdated(VectorTile),
    Updating(VectorTile),
    Packed(VectorTile),
    Error,
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
