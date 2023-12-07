use crate::layer::tile_provider::TileSource;
use crate::layer::vector_tile::style::VectorTileStyle;
use crate::layer::vector_tile::vector_tile::VectorTile;
use crate::messenger::Messenger;
use crate::render::Renderer;
use crate::tile_scheme::{TileIndex, TileScheme};
use maybe_sync::{MaybeSend, MaybeSync};
use std::collections::HashMap;
use std::sync::{Arc, RwLock, RwLockReadGuard};

#[cfg(target_arch = "wasm32")]
pub mod web_worker_provider;

#[cfg(not(target_arch = "wasm32"))]
pub mod rayon_provider;

pub trait VectorTileProvider: MaybeSend + MaybeSync {
    fn create(
        messenger: impl Messenger + 'static,
        tile_source: impl TileSource + 'static,
        tile_scheme: TileScheme,
    ) -> Self;

    fn supports(&self, renderer: &RwLock<dyn Renderer>) -> bool;
    fn load_tile(
        &self,
        index: TileIndex,
        style: &VectorTileStyle,
        renderer: &Arc<RwLock<dyn Renderer>>,
    );
    fn update_style(&self);
    fn read(&self) -> LockedTileStore;
}

pub struct LockedTileStore<'a> {
    guard: RwLockReadGuard<'a, HashMap<TileIndex, TileState>>,
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
