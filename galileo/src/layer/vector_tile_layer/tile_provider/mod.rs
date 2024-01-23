use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::vector_tile_layer::vector_tile::VectorTile;
use crate::messenger::Messenger;
use crate::render::Renderer;
use crate::tile_scheme::TileIndex;
use maybe_sync::{MaybeSend, MaybeSync};
use quick_cache::unsync::Cache;
use std::sync::{Arc, MutexGuard, RwLock};

#[cfg(target_arch = "wasm32")]
pub mod web_worker_provider;

#[cfg(not(target_arch = "wasm32"))]
pub mod rayon_provider;

pub mod vt_processor;

pub trait VectorTileProvider: MaybeSend + MaybeSync {
    fn supports(&self, renderer: &RwLock<dyn Renderer>) -> bool;
    fn load_tile(
        &self,
        index: TileIndex,
        style: &VectorTileStyle,
        renderer: &Arc<RwLock<dyn Renderer>>,
    );
    fn update_style(&self);
    fn read(&self) -> LockedTileStore;
    fn set_messenger(&self, messenger: Box<dyn Messenger>);
}

pub struct LockedTileStore<'a> {
    guard: MutexGuard<'a, Cache<TileIndex, TileState>>,
}

impl<'a> LockedTileStore<'a> {
    pub fn get_tile(&'a self, index: TileIndex) -> Option<&'a VectorTile> {
        self.guard.get(&index).and_then(|v| match v {
            TileState::Loaded(tile) | TileState::Updating(tile, _) => Some(tile),
            _ => None,
        })
    }
}

pub enum TileState {
    Loading(Arc<RwLock<dyn Renderer>>),
    Loaded(VectorTile),
    Outdated(VectorTile),
    Updating(VectorTile, Arc<RwLock<dyn Renderer>>),
    Error,
}
