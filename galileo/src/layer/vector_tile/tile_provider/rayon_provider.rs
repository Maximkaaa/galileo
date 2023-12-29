use crate::error::GalileoError;
use crate::layer::tile_provider::TileSource;
use crate::layer::vector_tile::style::VectorTileStyle;
use crate::layer::vector_tile::tile_provider::{LockedTileStore, TileState, VectorTileProvider};
use crate::layer::vector_tile::vector_tile::VectorTile;
use crate::messenger::Messenger;
use crate::platform::{PlatformService, PlatformServiceImpl};
use crate::render::Renderer;
use crate::tile_scheme::{TileIndex, TileScheme};
use bytes::Bytes;
use galileo_mvt::MvtTile;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct RayonProvider {
    messenger: Arc<RwLock<Option<Box<dyn Messenger>>>>,
    tile_source: Arc<dyn TileSource>,
    tile_schema: TileScheme,
    platform_service: PlatformServiceImpl,
    tiles: Arc<RwLock<HashMap<TileIndex, TileState>>>,
}

impl VectorTileProvider for RayonProvider {
    fn create(
        messenger: Option<Box<dyn Messenger>>,
        tile_source: impl TileSource + 'static,
        tile_scheme: TileScheme,
    ) -> Self {
        let platform_service = PlatformServiceImpl::new();
        Self {
            platform_service,
            messenger: Arc::new(RwLock::new(messenger)),
            tile_source: Arc::new(tile_source),
            tile_schema: tile_scheme,
            tiles: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn supports(&self, _renderer: &RwLock<dyn Renderer>) -> bool {
        true
    }

    fn load_tile(
        &self,
        index: TileIndex,
        style: &VectorTileStyle,
        renderer: &Arc<RwLock<dyn Renderer>>,
    ) {
        let mut tiles = self.tiles.write().unwrap();
        match tiles.get_mut(&index) {
            None => {
                tiles.insert(index, TileState::Loading(renderer.clone()));
            }
            Some(state @ TileState::Outdated(..)) => {
                let TileState::Outdated(tile) = std::mem::replace(state, TileState::Error) else {
                    panic!("Type of value changed unexpectingly");
                };
                *state = TileState::Updating(tile, renderer.clone());
            }
            _ => {
                return;
            }
        }

        self.load_tile_async(index, style);
    }

    fn update_style(&self) {
        let mut tiles = self.tiles.write().unwrap();
        for (_, tile_state) in tiles.iter_mut() {
            if matches!(tile_state, TileState::Loaded(_)) {
                let TileState::Loaded(tile) = std::mem::replace(tile_state, TileState::Error)
                else {
                    panic!("Type of value changed unexpectingly");
                };
                *tile_state = TileState::Outdated(tile);
            }
        }

        if let Some(messenger) = &(*self.messenger.read().unwrap()) {
            messenger.request_redraw();
        }
    }

    fn read(&self) -> LockedTileStore {
        LockedTileStore {
            guard: self.tiles.read().unwrap(),
        }
    }

    fn set_messenger(&self, messenger: Box<dyn Messenger>) {
        *self.messenger.write().unwrap() = Some(messenger.into())
    }
}

impl RayonProvider {
    fn load_tile_async(&self, index: TileIndex, style: &VectorTileStyle) {
        let provider = self.clone();
        let style = style.clone();
        crate::async_runtime::spawn(async move { provider.prepare_tile(index, style).await });
    }

    async fn prepare_tile(self, index: TileIndex, style: VectorTileStyle) {
        let bytes = match self.download_tile(index).await {
            Ok(bytes) => bytes,
            Err(e) => {
                log::info!("Error while loading tile {index:?}: {e:?}");
                let mut tile_store = self.tiles.write().unwrap();
                if !matches!(tile_store.get(&index), Some(TileState::Outdated(..))) {
                    tile_store.insert(index, TileState::Error);
                }

                return;
            }
        };

        rayon::spawn(move || match self.try_prepare_tile(bytes, index, &style) {
            Ok(tile) => {
                let mut tile_store = self.tiles.write().unwrap();
                if matches!(tile_store.get(&index), Some(TileState::Outdated(..))) {
                    log::info!("Tile {index:?} is loaded, but is not needed. Dropped.");
                } else {
                    tile_store.insert(index, TileState::Loaded(tile));
                    if let Some(messenger) = &(*self.messenger.read().unwrap()) {
                        messenger.request_redraw();
                    }
                }
            }
            Err(e) => {
                log::info!("Error while loading tile {index:?}: {e:?}");
                let mut tile_store = self.tiles.write().unwrap();
                if !matches!(tile_store.get(&index), Some(TileState::Outdated(..))) {
                    tile_store.insert(index, TileState::Error);
                }
            }
        });
    }

    fn try_prepare_tile(
        &self,
        bytes: Bytes,
        index: TileIndex,
        style: &VectorTileStyle,
    ) -> Result<VectorTile, GalileoError> {
        let mvt_tile = MvtTile::decode(bytes, false)?;

        match self.tiles.read().unwrap().get(&index) {
            Some(TileState::Loading(renderer) | TileState::Updating(_, renderer)) => {
                let renderer = renderer.read().unwrap();
                let tile =
                    VectorTile::create(mvt_tile, &*renderer, index, style, &self.tile_schema)?;

                Ok(tile)
            }
            _ => Err(GalileoError::IO),
        }
    }

    async fn download_tile(&self, index: TileIndex) -> Result<Bytes, GalileoError> {
        let url = (self.tile_source)(index);
        self.platform_service.load_bytes_from_url(&url).await
    }
}
