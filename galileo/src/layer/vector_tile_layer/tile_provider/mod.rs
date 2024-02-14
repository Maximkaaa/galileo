//! Vector tile layer tile providers

use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::vector_tile_layer::vector_tile::VectorTile;
use crate::messenger::Messenger;
use crate::render::render_bundle::RenderBundle;
use crate::render::Canvas;
use crate::tile_scheme::TileIndex;
use galileo_mvt::MvtTile;
use maybe_sync::{MaybeSend, MaybeSync};
use quick_cache::unsync::Cache;
use std::sync::MutexGuard;

#[cfg(target_arch = "wasm32")]
mod web_worker_provider;
#[cfg(target_arch = "wasm32")]
pub use web_worker_provider::WebWorkerVectorTileProvider;

#[cfg(not(target_arch = "wasm32"))]
mod threaded_provider;
#[cfg(not(target_arch = "wasm32"))]
pub use threaded_provider::ThreadedProvider;

mod vt_processor;
pub use vt_processor::{VectorTileDecodeContext, VtProcessor};

/// Vector tile provider.
pub trait VectorTileProvider: MaybeSend + MaybeSync {
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
            TileState::Packed(tile) => Some(tile),
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
