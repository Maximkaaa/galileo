use crate::error::GalileoError;
use crate::layer::data_provider::DataProvider;
use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::vector_tile_layer::tile_provider::vt_processor::VectorTileDecodeContext;
use crate::layer::vector_tile_layer::tile_provider::{
    LockedTileStore, TileState, UnpackedVectorTile, VectorTileProviderT,
};
use crate::messenger::Messenger;
use crate::render::render_bundle::RenderBundle;
use crate::tile_scheme::{TileIndex, TileSchema};
use bytes::Bytes;
use galileo_mvt::MvtTile;
use maybe_sync::{MaybeSend, MaybeSync};
use quick_cache::unsync::Cache;
use std::sync::{Arc, Mutex, RwLock};

/// Provider that uses background threads to load, decode and pack vector tiles.
pub struct ThreadedProvider<Provider>
where
    Provider: DataProvider<TileIndex, (RenderBundle, MvtTile), VectorTileDecodeContext>
        + MaybeSend
        + MaybeSync
        + 'static,
{
    messenger: Arc<RwLock<Option<Box<dyn Messenger>>>>,
    tile_schema: TileSchema,
    data_provider: Arc<Provider>,
    tiles: Arc<Mutex<Cache<TileIndex, TileState>>>,
    empty_bundle: RenderBundle,
}

impl<Provider> Clone for ThreadedProvider<Provider>
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
            empty_bundle: self.empty_bundle.clone(),
        }
    }
}

impl<Provider> VectorTileProviderT for ThreadedProvider<Provider>
where
    Provider: DataProvider<TileIndex, (RenderBundle, MvtTile), VectorTileDecodeContext>
        + MaybeSend
        + MaybeSync
        + 'static,
{
    fn load_tile(&self, index: TileIndex, style: &VectorTileStyle) {
        if self.set_loading_state(index) {
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
                let TileState::Packed(tile) = std::mem::replace(tile_state, TileState::Error)
                else {
                    log::error!("Type of value changed unexpectedly during updating style.");
                    continue;
                };

                *tile_state = TileState::Outdated(tile);
            }
        }

        if let Some(messenger) = &(*self.messenger.read().expect("lock is poisoned")) {
            messenger.request_redraw();
        }
    }

    fn read(&self) -> LockedTileStore {
        LockedTileStore {
            guard: self.tiles.lock().expect("tile store mutex is poisoned"),
        }
    }

    fn set_messenger(&self, messenger: Box<dyn Messenger>) {
        *self.messenger.write().expect("lock is poisoned") = Some(messenger)
    }
}

impl<Provider> ThreadedProvider<Provider>
where
    Provider: DataProvider<TileIndex, (RenderBundle, MvtTile), VectorTileDecodeContext>
        + MaybeSend
        + MaybeSync
        + 'static,
{
    /// Creates a new provider.
    pub fn new(
        messenger: Option<Box<dyn Messenger>>,
        tile_scheme: TileSchema,
        data_provider: Provider,
        empty_bundle: RenderBundle,
    ) -> Self {
        Self {
            messenger: Arc::new(RwLock::new(messenger)),
            tile_schema: tile_scheme,
            data_provider: Arc::new(data_provider),
            tiles: Arc::new(Mutex::new(Cache::new(1000))),
            empty_bundle,
        }
    }

    fn set_loading_state(&self, index: TileIndex) -> bool {
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

                *value = TileState::Updating(tile);
            }
        } else {
            tiles.insert(index, TileState::Loading);
        }

        true
    }

    fn load_tile_internal(&self, index: TileIndex, style: &VectorTileStyle) {
        let provider: ThreadedProvider<Provider> = (*self).clone();
        let style = style.clone();
        crate::async_runtime::spawn(async move {
            match provider.clone().load_tile_async(index, style).await {
                Ok(tile) => {
                    let mut tiles = provider.tiles.lock().expect("tile store mutex is poisoned");
                    tiles.insert(index, TileState::Loaded(Box::new(tile)));
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
    ) -> Result<UnpackedVectorTile, GalileoError> {
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
    ) -> Result<UnpackedVectorTile, GalileoError> {
        let bundle = self.empty_bundle.clone();
        let context = VectorTileDecodeContext {
            index,
            style: style.clone(),
            tile_schema: self.tile_schema.clone(),
            bundle,
        };
        let (bundle, mvt_tile) = self.data_provider.decode(bytes, context)?;

        Ok(UnpackedVectorTile { bundle, mvt_tile })
    }

    async fn download_tile(&self, index: TileIndex) -> Result<Bytes, GalileoError> {
        self.data_provider.load_raw(&index).await
    }
}
