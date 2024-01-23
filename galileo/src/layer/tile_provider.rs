use crate::messenger::Messenger;
use crate::platform::{PlatformService, PlatformServiceImpl};
use crate::tile_scheme::TileIndex;
use async_trait::async_trait;
use maybe_sync::{MaybeSend, MaybeSync};
use quick_cache::sync::Cache;
use std::sync::{Arc, RwLock};

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait TileProvider<Tile>: MaybeSend + MaybeSync {
    fn get_tile(&self, index: TileIndex) -> Option<Arc<Tile>>;
    fn set_messenger(&self, messenger: Box<dyn Messenger>);
    fn request_redraw(&self);

    async fn load_tile(&self, index: TileIndex);
}

pub trait TileSource: (Fn(TileIndex) -> String) + MaybeSend + MaybeSync {}
impl<T: Fn(TileIndex) -> String> TileSource for T where T: MaybeSend + MaybeSync {}

pub struct UrlTileProvider<Tile> {
    pub url_source: Box<dyn TileSource>,
    pub platform_service: PlatformServiceImpl,
    pub messenger: RwLock<Option<Box<dyn Messenger>>>,
    pub tile_cache: Cache<TileIndex, TileState<Tile>>,
}

impl<Tile: Clone> UrlTileProvider<Tile> {
    pub fn new(url_source: Box<dyn TileSource>, messenger: Option<Box<dyn Messenger>>) -> Self {
        Self {
            url_source,
            platform_service: PlatformServiceImpl::new(),
            messenger: RwLock::new(messenger),
            tile_cache: Cache::new(5000),
        }
    }

    pub fn get_tile_int(&self, index: TileIndex) -> Option<Arc<Tile>> {
        match self.tile_cache.get(&index) {
            Some(TileState::Loaded(tile)) => Some(tile),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TileState<T> {
    Loading,
    Loaded(Arc<T>),
    Failed,
}
