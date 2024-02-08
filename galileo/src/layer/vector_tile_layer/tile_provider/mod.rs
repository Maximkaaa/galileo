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
pub mod web_worker_provider;

#[cfg(not(target_arch = "wasm32"))]
pub mod rayon_provider;

pub mod vt_processor;

pub trait VectorTileProvider: MaybeSend + MaybeSync {
    fn load_tile(&self, index: TileIndex, style: &VectorTileStyle);
    fn update_style(&self);
    fn read(&self) -> LockedTileStore;
    fn set_messenger(&self, messenger: Box<dyn Messenger>);
}

pub struct LockedTileStore<'a> {
    guard: MutexGuard<'a, Cache<TileIndex, TileState>>,
}

impl<'a> LockedTileStore<'a> {
    pub fn get_mvt_tile(&'a self, index: TileIndex) -> Option<&'a MvtTile> {
        self.guard.get(&index).and_then(|v| match v {
            TileState::Loaded(tile) => Some(&tile.mvt_tile),
            TileState::Packed(tile) | TileState::Updating(tile) | TileState::Outdated(tile) => {
                Some(&tile.mvt_tile)
            }
            _ => None,
        })
    }

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
