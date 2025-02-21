//! Raster tile layer and its providers

use std::any::Any;
use std::collections::HashSet;
use std::sync::Arc;

use galileo_types::cartesian::Size;
use parking_lot::Mutex;
use quick_cache::sync::Cache;
use web_time::{Duration, SystemTime};

use super::Layer;
use crate::attribution::Attribution;
use crate::decoded_image::DecodedImage;
use crate::messenger::Messenger;
use crate::render::{Canvas, ImagePaint, PackedBundle, RenderOptions};
use crate::tile_schema::{TileIndex, TileSchema};
use crate::view::MapView;

mod provider;
pub use provider::{RasterTileProvider, RestTileProvider};

mod builder;
pub use builder::RasterTileLayerBuilder;

/// Raster tile layers load prerendered tile sets using [tile provider](RasterTileProvider) and render them to the map.
pub struct RasterTileLayer {
    tile_provider: Arc<dyn RasterTileProvider>,
    tile_schema: TileSchema,
    fade_in_duration: Duration,
    tiles: Arc<Cache<TileIndex, Arc<TileState>>>,
    prev_drawn_tiles: Mutex<Vec<TileIndex>>,
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

enum TileState {
    Loading,
    Loaded(Mutex<DecodedImage>),
    Rendered(Box<Mutex<RenderedTile>>),
    Error,
}

struct RenderedTile {
    packed_bundle: Box<dyn PackedBundle>,
    first_drawn: SystemTime,
    opacity: f32,
}

impl RenderedTile {
    fn is_opaque(&self) -> bool {
        self.opacity > 0.999
    }
}

impl RasterTileLayer {
    /// Creates anew layer.
    pub fn new(
        tile_schema: TileSchema,
        tile_provider: impl RasterTileProvider + 'static,
        messenger: Option<Arc<dyn Messenger>>,
    ) -> Self {
        Self {
            tile_provider: Arc::new(tile_provider),
            tile_schema,
            prev_drawn_tiles: Mutex::new(vec![]),
            fade_in_duration: Duration::from_millis(300),
            tiles: Arc::new(Cache::new(5000)),
            messenger,
            attribution: None,
        }
    }

    fn new_raw(
        tile_provider: Box<dyn RasterTileProvider>,
        tile_schema: TileSchema,
        messenger: Option<Box<dyn Messenger>>,
    ) -> Self {
        Self {
            tile_provider: tile_provider.into(),
            tile_schema,
            prev_drawn_tiles: Mutex::new(vec![]),
            fade_in_duration: Duration::from_millis(300),
            tiles: Arc::new(Cache::new(5000)),
            messenger: messenger.map(|m| m.into()),
            attribution: Some(Attribution::new(
                "© OpenStreetMap contributors",
                Some("https://www.openstreetmap.org/copyright"),
            )),
        }
    }

    /// Sets fade in duration for newly loaded tiles.
    pub fn set_fade_in_duration(&mut self, duration: Duration) {
        self.fade_in_duration = duration;
    }

    fn get_tiles_to_draw(&self, view: &MapView) -> Vec<(TileIndex, Arc<TileState>)> {
        let mut tiles = vec![];
        let Some(tile_iter) = self.tile_schema.iter_tiles(view) else {
            return vec![];
        };

        let mut to_substitute = vec![];
        for index in tile_iter {
            self.tiles.get(&index);

            match self.tiles.get(&index) {
                None => to_substitute.push(index),
                Some(tile_state) => match &*tile_state.clone() {
                    TileState::Rendered(tile) => {
                        if !tile.lock().is_opaque() {
                            to_substitute.push(index);
                        }

                        tiles.push((index, tile_state));
                    }
                    TileState::Loaded(_) => {
                        to_substitute.push(index);
                        tiles.push((index, tile_state));
                    }
                    _ => to_substitute.push(index),
                },
            }
        }

        let prev_drawn = self.prev_drawn_tiles.lock();
        let mut substitute_indices: HashSet<_> = tiles.iter().map(|(index, _)| *index).collect();
        let mut substitute_tiles = vec![];
        for index in to_substitute {
            let mut next_level = index;
            let mut substituted = false;

            while let Some(subst) = self.tile_schema.get_substitutes(next_level) {
                let mut need_more = false;
                for substitute_index in subst {
                    // todo: this will not work correctly if a tile is substituted by more then 1 tile
                    next_level = substitute_index;

                    if let Some(tile) = self.tiles.get(&substitute_index) {
                        if matches!(*tile, TileState::Rendered(_))
                            && !substitute_indices.contains(&substitute_index)
                        {
                            substitute_tiles.push((substitute_index, tile));
                            substitute_indices.insert(substitute_index);
                        }

                        if let Some(TileState::Rendered(rendered)) = self
                            .tiles
                            .get(&substitute_index)
                            .as_ref()
                            .map(|v| v.as_ref())
                        {
                            if !rendered.lock().is_opaque() {
                                need_more = true;
                            }
                        }
                    } else {
                        need_more = true;
                    }
                }

                if !need_more {
                    substituted = true;
                    break;
                }
            }

            if !substituted {
                let Some(required_bbox) = self.tile_schema.tile_bbox(index) else {
                    continue;
                };
                for prev in prev_drawn.iter() {
                    let Some(prev_bbox) = self.tile_schema.tile_bbox(*prev) else {
                        continue;
                    };
                    if !substitute_indices.contains(prev) && prev_bbox.intersects(required_bbox) {
                        substitute_indices.insert(*prev);
                        let Some(tile) = self.tiles.get(prev) else {
                            continue;
                        };
                        substitute_tiles.push((*prev, tile));
                    }
                }
            }
        }

        substitute_tiles.sort_unstable_by(|(index_a, _), (index_b, _)| index_a.z.cmp(&index_b.z));
        substitute_tiles.append(&mut tiles);
        substitute_tiles.dedup_by(|a, b| a.0 == b.0);
        substitute_tiles
    }

    fn prepare_tile_renders(&self, tiles: &[(TileIndex, Arc<TileState>)], canvas: &mut dyn Canvas) {
        let mut requires_redraw = false;

        let now = SystemTime::now();
        for (index, tile) in tiles {
            match &**tile {
                TileState::Rendered(rendered) => {
                    let mut rendered = rendered.lock();
                    if rendered.is_opaque() {
                        continue;
                    }

                    let first_drawn = rendered.first_drawn;

                    let since_drawn = now
                        .duration_since(first_drawn)
                        .unwrap_or(Duration::from_millis(0));
                    let opacity = if self.fade_in_duration.is_zero() {
                        0.0
                    } else {
                        (since_drawn.as_secs_f64() / self.fade_in_duration.as_secs_f64()).min(1.0)
                            as f32
                    };

                    rendered.opacity = opacity;
                    if !rendered.is_opaque() {
                        requires_redraw = true;
                    }
                }
                TileState::Loaded(decoded_image) => {
                    let mut bundle = canvas.create_bundle();
                    let mut decoded_image = decoded_image.lock();

                    let owned = std::mem::replace(
                        &mut *decoded_image,
                        DecodedImage::from_raw(vec![], Size::new(0, 0))
                            .expect("empty image is always ok"),
                    );

                    let opacity = if self.fade_in_duration.is_zero() {
                        1.0
                    } else {
                        0.0
                    };

                    let Some(tile_bbox) = self.tile_schema.tile_bbox(*index) else {
                        log::warn!("Failed to get bbox for tile {index:?}");
                        continue;
                    };

                    bundle.add_image(
                        owned,
                        tile_bbox.into_quadrangle(),
                        ImagePaint { opacity: 255 },
                    );
                    let packed = canvas.pack_bundle(&bundle);
                    self.tiles.insert(
                        *index,
                        Arc::new(TileState::Rendered(Box::new(Mutex::new(RenderedTile {
                            packed_bundle: packed,
                            first_drawn: now,
                            opacity,
                        })))),
                    );

                    requires_redraw = true;
                }
                _ => {}
            }
        }

        if requires_redraw {
            if let Some(messenger) = &self.messenger {
                messenger.request_redraw();
            }
        }
    }

    async fn load_tile(
        index: TileIndex,
        tile_provider: Arc<dyn RasterTileProvider>,
        tiles: &Cache<TileIndex, Arc<TileState>>,
        messenger: Option<Arc<dyn Messenger>>,
    ) {
        match tiles.get_value_or_guard_async(&index).await {
            Ok(_) => {}
            Err(guard) => {
                let _ = guard.insert(Arc::new(TileState::Loading));
                let load_result = tile_provider.load(index).await;

                match load_result {
                    Ok(decoded_image) => {
                        if let Some(v) = tiles.get(&index) {
                            if matches!(*v, TileState::Rendered(_)) {
                                log::error!("This should not happen to {index:?}");
                            }
                        }

                        tiles.insert(
                            index,
                            Arc::new(TileState::Loaded(Mutex::new(decoded_image))),
                        );

                        if let Some(messenger) = messenger {
                            messenger.request_redraw();
                        }
                    }
                    Err(err) => {
                        log::debug!("Failed to load tile: {err}");
                        tiles.insert(index, Arc::new(TileState::Error))
                    }
                }
            }
        }
    }

    /// Preload tiles for the given `view`.
    pub async fn load_tiles(&self, view: &MapView) {
        if let Some(iter) = self.tile_schema.iter_tiles(view) {
            for index in iter {
                let tile_provider = self.tile_provider.clone();
                let tiles = self.tiles.clone();
                let messenger = self.messenger.clone();
                Self::load_tile(index, tile_provider, &tiles, messenger).await;
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
        let tiles = self.get_tiles_to_draw(view);
        self.prepare_tile_renders(&tiles, canvas);

        let updated_tiles: Vec<_> = tiles
            .iter()
            .filter_map(|(index, _)| self.tiles.get(index))
            .collect();
        let mut to_draw = Vec::new();
        for tile in &updated_tiles {
            if let TileState::Rendered(rendered) = tile.as_ref() {
                to_draw.push(rendered.lock());
            }
        }

        canvas.draw_bundles_with_opacity(
            &to_draw
                .iter()
                .map(|guard| (&*guard.packed_bundle, guard.opacity))
                .collect::<Vec<_>>(),
            RenderOptions::default(),
        );
        *self.prev_drawn_tiles.lock() = tiles.iter().map(|(index, _)| *index).collect();
    }

    fn prepare(&self, view: &MapView) {
        if let Some(iter) = self.tile_schema.iter_tiles(view) {
            for index in iter {
                let tile_provider = self.tile_provider.clone();
                let tiles = self.tiles.clone();
                let messenger = self.messenger.clone();
                crate::async_runtime::spawn(async move {
                    Self::load_tile(index, tile_provider, &tiles, messenger).await;
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
