use crate::error::GalileoError;
use crate::layer::data_provider::DataProvider;
use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::vector_tile_layer::tile_provider::vt_processor::VectorTileDecodeContext;
use crate::layer::vector_tile_layer::tile_provider::{
    LockedTileStore, TileState, VectorTileProvider,
};
use crate::layer::vector_tile_layer::vector_tile::VectorTile;
use crate::messenger::Messenger;
use crate::render::render_bundle::RenderBundle;
use crate::render::Renderer;
use crate::tile_scheme::{TileIndex, TileScheme};
use bytes::Bytes;
use galileo_mvt::MvtTile;
use maybe_sync::{MaybeSend, MaybeSync};
use quick_cache::unsync::Cache;
use std::sync::{Arc, Mutex, RwLock};

pub struct RayonProvider<Provider>
where
    Provider: DataProvider<TileIndex, (RenderBundle, MvtTile), VectorTileDecodeContext>
        + MaybeSend
        + MaybeSync
        + 'static,
{
    messenger: Arc<RwLock<Option<Box<dyn Messenger>>>>,
    tile_schema: TileScheme,
    data_provider: Arc<Provider>,
    tiles: Arc<Mutex<Cache<TileIndex, TileState>>>,
}

impl<Provider> Clone for RayonProvider<Provider>
where
    Provider: DataProvider<TileIndex, (RenderBundle, MvtTile), VectorTileDecodeContext>
        + MaybeSend
        + MaybeSync
        + 'static,
{
    fn clone(&self) -> Self {
        Self {
            messenger: self.messenger.clone(),
            tile_schema: self.tile_schema.clone(),
            data_provider: self.data_provider.clone(),
            tiles: self.tiles.clone(),
        }
    }
}

impl<Provider> VectorTileProvider for RayonProvider<Provider>
where
    Provider: DataProvider<TileIndex, (RenderBundle, MvtTile), VectorTileDecodeContext>
        + MaybeSend
        + MaybeSync
        + 'static,
{
    fn supports(&self, _renderer: &RwLock<dyn Renderer>) -> bool {
        true
    }

    fn load_tile(
        &self,
        index: TileIndex,
        style: &VectorTileStyle,
        renderer: &Arc<RwLock<dyn Renderer>>,
    ) {
        if self.set_loading_state(index, renderer) {
            self.load_tile_internal(index, style);
        }
    }

    fn update_style(&self) {
        let mut tiles = self.tiles.lock().expect("tile store mutex is poisoned");
        let indices: Vec<_> = tiles.iter().map(|(index, _)| *index).collect();

        for index in indices {
            let Some(mut entry) = tiles.get_mut(&index) else {
                continue;
            };
            let tile_state = &mut *entry;
            if matches!(*tile_state, TileState::Loaded(_)) {
                let TileState::Loaded(tile) = std::mem::replace(tile_state, TileState::Error)
                else {
                    log::error!("Type of value changed unexpectedly during updating style.");
                    continue;
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
            guard: self.tiles.lock().expect("tile store mutex is poisoned"),
        }
    }

    fn set_messenger(&self, messenger: Box<dyn Messenger>) {
        *self.messenger.write().unwrap() = Some(messenger)
    }
}

impl<Provider> RayonProvider<Provider>
where
    Provider: DataProvider<TileIndex, (RenderBundle, MvtTile), VectorTileDecodeContext>
        + MaybeSend
        + MaybeSync
        + 'static,
{
    pub fn new(
        messenger: Option<Box<dyn Messenger>>,
        tile_scheme: TileScheme,
        data_provider: Provider,
    ) -> Self {
        Self {
            messenger: Arc::new(RwLock::new(messenger)),
            tile_schema: tile_scheme,
            data_provider: Arc::new(data_provider),
            tiles: Arc::new(Mutex::new(Cache::new(1000))),
        }
    }

    fn set_loading_state(&self, index: TileIndex, renderer: &Arc<RwLock<dyn Renderer>>) -> bool {
        let mut tiles = self.tiles.lock().expect("tile store mutex is poisoned");
        let has_entry = tiles.peek(&index).is_some();
        if has_entry {
            if let Some(mut entry) = tiles.get_mut(&index) {
                let value = &mut *entry;
                if !matches!(value, TileState::Outdated(..)) {
                    return false;
                }

                let TileState::Outdated(tile) = std::mem::replace(value, TileState::Error) else {
                    log::error!("Type of value changed unexpectedly during loading.");
                    return false;
                };

                *value = TileState::Updating(tile, renderer.clone());
            }
        } else {
            tiles.insert(index, TileState::Loading(renderer.clone()));
        }

        true
    }

    fn load_tile_internal(&self, index: TileIndex, style: &VectorTileStyle) {
        let provider: RayonProvider<Provider> = (*self).clone();
        let style = style.clone();
        crate::async_runtime::spawn(async move {
            match provider.clone().load_tile_async(index, style).await {
                Ok(tile) => {
                    let mut tiles = provider.tiles.lock().expect("tile store mutex is poisoned");
                    tiles.insert(index, TileState::Loaded(tile));
                    if let Some(messenger) = &*provider
                        .messenger
                        .read()
                        .expect("messenger mutex is poisoned")
                    {
                        messenger.request_redraw();
                    }
                }
                Err(err) => {
                    log::info!("Failed to load tile: {err:?}");
                    let mut tiles = provider.tiles.lock().expect("tile store mutex is poisoned");
                    tiles.insert(index, TileState::Error);
                }
            }
        });
    }

    async fn load_tile_async(
        self,
        index: TileIndex,
        style: VectorTileStyle,
    ) -> Result<VectorTile, GalileoError> {
        let bytes = self.download_tile(index).await?;
        tokio::task::spawn_blocking(move || self.try_prepare_tile(bytes, index, &style))
            .await
            .unwrap_or_else(|err| {
                Err(GalileoError::Generic(format!(
                    "Failed to load tile: {err:?}"
                )))
            })
    }

    fn try_prepare_tile(
        &self,
        bytes: Bytes,
        index: TileIndex,
        style: &VectorTileStyle,
    ) -> Result<VectorTile, GalileoError> {
        let renderer = {
            let mut tiles = self.tiles.lock().expect("tile store mutex is poisoned");
            match tiles.get(&index) {
                Some(TileState::Loading(renderer)) | Some(TileState::Updating(_, renderer)) => {
                    renderer.clone()
                }
                _ => {
                    tiles.remove(&index);
                    return Err(GalileoError::Generic("tile was in invalid state".into()));
                }
            }

            // drop mutex before doing expensive computations
        };

        let renderer = renderer.read().expect("renderer lock is poisoned");

        let bundle = renderer.create_bundle();
        let context = VectorTileDecodeContext {
            index,
            style: style.clone(),
            tile_scheme: self.tile_schema.clone(),
            bundle,
        };
        let (bundle, mvt_tile) = self.data_provider.decode(bytes, context)?;
        let packed_bundle = renderer.pack_bundle(&bundle);

        Ok(VectorTile {
            bundle: packed_bundle,
            mvt_tile,
        })
    }

    async fn download_tile(&self, index: TileIndex) -> Result<Bytes, GalileoError> {
        self.data_provider.load_raw(&index).await
    }
}
