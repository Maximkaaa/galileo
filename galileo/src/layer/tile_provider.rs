use crate::error::GalileoError;
use crate::messenger::Messenger;
use crate::platform::{PlatformService, PlatformServiceImpl};
use crate::tile_scheme::TileIndex;
use async_trait::async_trait;
use maybe_sync::{MaybeSend, MaybeSync};
use std::collections::HashMap;
use std::sync::RwLock;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait TileProvider<Tile>: MaybeSend + MaybeSync {
    fn get_tile(&self, index: TileIndex) -> Option<Tile>;
    async fn load_tile(&self, index: TileIndex) -> Result<(), GalileoError>;
    fn set_messenger(&self, messenger: Box<dyn Messenger>);
}

pub trait TileSource: (Fn(TileIndex) -> String) + MaybeSend + MaybeSync {}
impl<T: Fn(TileIndex) -> String> TileSource for T where T: MaybeSend + MaybeSync {}

pub struct UrlTileProvider<Tile> {
    pub url_source: Box<dyn TileSource>,
    pub platform_service: PlatformServiceImpl,
    pub loaded_tiles: RwLock<HashMap<TileIndex, TileState<Tile>>>,
    pub messenger: RwLock<Option<Box<dyn Messenger>>>,
}

impl<Tile: Clone> UrlTileProvider<Tile> {
    pub fn new(url_source: Box<dyn TileSource>, messenger: Option<Box<dyn Messenger>>) -> Self {
        Self {
            url_source,
            platform_service: PlatformServiceImpl::new(),
            loaded_tiles: RwLock::new(HashMap::new()),
            messenger: RwLock::new(messenger),
        }
    }

    pub fn get_tile_int(&self, index: TileIndex) -> Option<Tile> {
        self.loaded_tiles
            .read()
            .unwrap()
            .get(&index)
            .cloned()
            .and_then(|v| v.get())
    }
}

#[derive(Debug, Clone)]
pub enum TileState<T> {
    Loading,
    Loaded(T),
    Failed,
}

impl<T> TileState<T> {
    pub fn get(self) -> Option<T> {
        match self {
            TileState::Loaded(v) => Some(v),
            _ => None,
        }
    }
}
