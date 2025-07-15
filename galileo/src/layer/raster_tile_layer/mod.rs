//! Raster tile layer and its providers

use std::any::Any;
use std::sync::Arc;

use provider::RasterTileProvider;
use web_time::Duration;

use super::tiles::TilesContainer;
use super::Layer;
use crate::layer::attribution::Attribution;
use crate::messenger::Messenger;
use crate::render::{Canvas, RenderOptions};
use crate::tile_schema::{TileIndex, TileSchema};
use crate::view::MapView;

mod provider;
pub use provider::{RasterTileLoader, RestTileLoader};

mod builder;
pub use builder::RasterTileLayerBuilder;

/// Raster tile layers load prerendered tile sets using [tile loader](RasterTileLoader) and render them to the map.
pub struct RasterTileLayer {
    tile_loader: Arc<dyn RasterTileLoader>,
    tile_container: Arc<TilesContainer<(), RasterTileProvider>>,
    tile_schema: TileSchema,
    fade_in_duration: Duration,
    messenger: Option<Arc<dyn Messenger>>,
    attribution: Option<Attribution>,
}

impl std::fmt::Debug for RasterTileLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RasterTileLayer")
            .field("tile_schema", &self.tile_schema)
            .field("fade_in_duration", &self.fade_in_duration)
            .finish()
    }
}

impl RasterTileLayer {
    /// Creates anew layer.
    pub fn new(
        tile_schema: TileSchema,
        tile_loader: impl RasterTileLoader + 'static,
        messenger: Option<Arc<dyn Messenger>>,
    ) -> Self {
        Self {
            tile_loader: Arc::new(tile_loader),
            tile_container: Arc::new(TilesContainer::new(
                tile_schema.clone(),
                RasterTileProvider::new(tile_schema.clone()),
            )),
            tile_schema,
            fade_in_duration: Duration::from_millis(300),
            messenger,
            attribution: None,
        }
    }

    fn new_raw(
        tile_loader: Box<dyn RasterTileLoader>,
        tile_schema: TileSchema,
        messenger: Option<Box<dyn Messenger>>,
        attribution: Option<Attribution>,
    ) -> Self {
        Self {
            tile_loader: tile_loader.into(),
            tile_container: Arc::new(TilesContainer::new(
                tile_schema.clone(),
                RasterTileProvider::new(tile_schema.clone()),
            )),
            tile_schema,
            fade_in_duration: Duration::from_millis(300),
            messenger: messenger.map(|m| m.into()),
            attribution,
        }
    }

    /// Sets fade in duration for newly loaded tiles.
    pub fn set_fade_in_duration(&mut self, duration: Duration) {
        self.fade_in_duration = duration;
    }

    fn update_displayed_tiles(&self, view: &MapView, canvas: &dyn Canvas) {
        let Some(tile_iter) = self.tile_schema.iter_tiles(view) else {
            return;
        };

        let needed_indices: Vec<_> = tile_iter.collect();
        self.tile_container
            .tile_provider
            .pack_tiles(&needed_indices, canvas);
        let requires_redraw = self
            .tile_container
            .update_displayed_tiles(needed_indices, ());

        if requires_redraw {
            if let Some(messenger) = &self.messenger {
                messenger.request_redraw();
            }
        }
    }

    async fn load_tile(
        index: TileIndex,
        tile_loader: Arc<dyn RasterTileLoader>,
        tiles: Arc<TilesContainer<(), RasterTileProvider>>,
        messenger: Option<Arc<dyn Messenger>>,
    ) {
        if tiles.tile_provider.set_loading(index) {
            // Already loading
            return;
        }

        let load_result = tile_loader.load(index).await;

        match load_result {
            Ok(decoded_image) => {
                tiles.tile_provider.set_loaded(index, decoded_image);

                if let Some(messenger) = messenger {
                    messenger.request_redraw();
                }
            }
            Err(err) => {
                log::debug!("Failed to load tile: {err}");
                tiles.tile_provider.set_error(index);
            }
        }
    }

    /// Preload tiles for the given `view`.
    pub async fn load_tiles(&self, view: &MapView) {
        if let Some(iter) = self.tile_schema.iter_tiles(view) {
            for index in iter {
                let tile_provider = self.tile_loader.clone();
                let messenger = self.messenger.clone();
                Self::load_tile(index, tile_provider, self.tile_container.clone(), messenger).await;
            }
        }
    }

    /// Returns tile schema of the layer.
    pub fn tile_schema(&self) -> &TileSchema {
        &self.tile_schema
    }
}

impl Layer for RasterTileLayer {
    fn render(&self, view: &MapView, canvas: &mut dyn Canvas) {
        self.update_displayed_tiles(view, canvas);

        let displayed_tiles = self.tile_container.tiles.lock();
        let to_render: Vec<_> = displayed_tiles
            .iter()
            .map(|v| (&*v.bundle, v.opacity))
            .collect();

        canvas.draw_bundles_with_opacity(&to_render, RenderOptions::default());
    }

    fn prepare(&self, view: &MapView, _canvas: &mut dyn Canvas) {
        if let Some(iter) = self.tile_schema.iter_tiles(view) {
            for index in iter {
                let tile_provider = self.tile_loader.clone();
                let container = self.tile_container.clone();
                let messenger = self.messenger.clone();
                crate::async_runtime::spawn(async move {
                    Self::load_tile(index, tile_provider, container, messenger).await;
                });
            }
        }
    }

    fn set_messenger(&mut self, messenger: Box<dyn Messenger>) {
        self.messenger = Some(Arc::from(messenger));
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn tile_schema(&self) -> Option<TileSchema> {
        Some(self.tile_schema.clone())
    }

    fn attribution(&self) -> Option<Attribution> {
        self.attribution.clone()
    }
}
