use crate::decoded_image::DecodedImage;
use crate::layer::data_provider::DataProvider;
use crate::messenger::Messenger;
use crate::render::render_bundle::RenderBundle;
use crate::render::{Canvas, ImagePaint, PackedBundle, PrimitiveId, RenderOptions};
use crate::tile_scheme::{TileIndex, TileSchema};
use crate::view::MapView;
use maybe_sync::{MaybeSend, MaybeSync, Mutex};
use quick_cache::sync::Cache;
use std::any::Any;
use std::collections::HashSet;
use std::sync::Arc;
use web_time::{Duration, SystemTime};

use super::Layer;

/// Raster tile layers load prerender tile sets using [`Provider`](DataProvider) and render them to the map.
pub struct RasterTileLayer<Provider>
where
    Provider: DataProvider<TileIndex, DecodedImage, ()> + MaybeSync + MaybeSend,
{
    tile_provider: Arc<Provider>,
    tile_scheme: TileSchema,
    fade_in_duration: Duration,
    tiles: Arc<Cache<TileIndex, Arc<TileState>>>,
    prev_drawn_tiles: Mutex<Vec<TileIndex>>,
    messenger: Option<Arc<dyn Messenger>>,
}

enum TileState {
    Loading,
    Loaded(Mutex<DecodedImage>),
    Rendered(Box<Mutex<RenderedTile>>),
    Error,
}

struct RenderedTile {
    render_bundle: RenderBundle,
    packed_bundle: Box<dyn PackedBundle>,
    first_drawn: SystemTime,
    is_opaque: bool,
    primitive_id: PrimitiveId,
}

impl<Provider> RasterTileLayer<Provider>
where
    Provider: DataProvider<TileIndex, DecodedImage, ()> + MaybeSync + MaybeSend,
{
    /// Creates anew layer.
    pub fn new(
        tile_scheme: TileSchema,
        tile_provider: Provider,
        messenger: Option<Arc<dyn Messenger>>,
    ) -> Self {
        Self {
            tile_provider: Arc::new(tile_provider),
            tile_scheme,
            prev_drawn_tiles: Mutex::new(vec![]),
            fade_in_duration: Duration::from_millis(300),
            tiles: Arc::new(Cache::new(5000)),
            messenger,
        }
    }

    /// Sets fade in duration for newly loaded tiles.
    pub fn set_fade_in_duration(&mut self, duration: Duration) {
        self.fade_in_duration = duration;
    }

    fn get_tiles_to_draw(&self, view: &MapView) -> Vec<(TileIndex, Arc<TileState>)> {
        let mut tiles = vec![];
        let Some(tile_iter) = self.tile_scheme.iter_tiles(view) else {
            return vec![];
        };

        let mut to_substitute = vec![];
        for index in tile_iter {
            self.tiles.get(&index);

            match self.tiles.get(&index) {
                None => to_substitute.push(index),
                Some(tile_state) => match &*tile_state.clone() {
                    TileState::Rendered(tile) => {
                        if !tile.lock().is_opaque {
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

            while let Some(subst) = self.tile_scheme.get_substitutes(next_level) {
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
                            if !rendered.lock().is_opaque {
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
                let Some(required_bbox) = self.tile_scheme.tile_bbox(index) else {
                    continue;
                };
                for prev in prev_drawn.iter() {
                    let Some(prev_bbox) = self.tile_scheme.tile_bbox(*prev) else {
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
                    if rendered.is_opaque {
                        continue;
                    }

                    let first_drawn = rendered.first_drawn;
                    let primitive_id = rendered.primitive_id;

                    let since_drawn = now
                        .duration_since(first_drawn)
                        .unwrap_or(Duration::from_millis(0));
                    let opacity = if self.fade_in_duration.is_zero() {
                        255
                    } else {
                        ((since_drawn.as_secs_f64() / self.fade_in_duration.as_secs_f64()).min(1.0)
                            * 255.0) as u8
                    };

                    let is_opaque = opacity == 255;
                    if !is_opaque {
                        requires_redraw = true;
                    }

                    if let Err(err) = rendered
                        .render_bundle
                        .modify_image(primitive_id, ImagePaint { opacity })
                    {
                        log::warn!("Failed to update image style: {err}");
                    }

                    let packed = canvas.pack_bundle(&rendered.render_bundle);
                    rendered.packed_bundle = packed;
                    rendered.is_opaque = is_opaque;
                }
                TileState::Loaded(decoded_image) => {
                    let mut bundle = canvas.create_bundle();
                    let mut decoded_image = decoded_image.lock();

                    let owned = std::mem::replace(
                        &mut *decoded_image,
                        DecodedImage::from_raw(vec![], 0, 0).expect("empty image is always ok"),
                    );

                    let opacity = if self.fade_in_duration.is_zero() {
                        255
                    } else {
                        0
                    };

                    let Some(tile_bbox) = self.tile_scheme.tile_bbox(*index) else {
                        log::warn!("Failed to get bbox for tile {index:?}");
                        continue;
                    };

                    let id = bundle.add_image(
                        owned,
                        tile_bbox.into_quadrangle(),
                        ImagePaint { opacity },
                    );
                    let packed = canvas.pack_bundle(&bundle);
                    self.tiles.insert(
                        *index,
                        Arc::new(TileState::Rendered(Box::new(Mutex::new(RenderedTile {
                            render_bundle: bundle,
                            packed_bundle: packed,
                            first_drawn: now,
                            is_opaque: false,
                            primitive_id: id,
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
        tile_provider: Arc<Provider>,
        tiles: &Cache<TileIndex, Arc<TileState>>,
        messenger: Option<Arc<dyn Messenger>>,
    ) {
        match tiles.get_value_or_guard_async(&index).await {
            Ok(_) => {}
            Err(guard) => {
                let _ = guard.insert(Arc::new(TileState::Loading));
                let load_result = tile_provider.load(&index, ()).await;

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
                    Err(_) => tiles.insert(index, Arc::new(TileState::Error)),
                }
            }
        }
    }

    /// Preload tiles for the given `view`.
    pub async fn load_tiles(&self, view: &MapView) {
        if let Some(iter) = self.tile_scheme.iter_tiles(view) {
            for index in iter {
                let tile_provider = self.tile_provider.clone();
                let tiles = self.tiles.clone();
                let messenger = self.messenger.clone();
                Self::load_tile(index, tile_provider, &tiles, messenger).await;
            }
        }
    }
}

impl<Provider> Layer for RasterTileLayer<Provider>
where
    Provider: DataProvider<TileIndex, DecodedImage, ()> + MaybeSync + MaybeSend + 'static,
{
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

        canvas.draw_bundles(
            &to_draw
                .iter()
                .map(|guard| &*guard.packed_bundle)
                .collect::<Vec<_>>(),
            RenderOptions::default(),
        );
        *self.prev_drawn_tiles.lock() = tiles.iter().map(|(index, _)| *index).collect();
    }

    fn prepare(&self, view: &MapView) {
        if let Some(iter) = self.tile_scheme.iter_tiles(view) {
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
}
