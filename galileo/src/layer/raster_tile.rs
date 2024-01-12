use crate::error::GalileoError;
use crate::layer::tile_provider::{TileProvider, TileSource, TileState, UrlTileProvider};
use crate::messenger::Messenger;
use crate::platform::PlatformService;
use crate::primitives::DecodedImage;
use crate::render::{Canvas, ImagePaint, PackedBundle, PrimitiveId, Renderer};
use crate::tile_scheme::{TileIndex, TileScheme};
use crate::view::MapView;
use async_trait::async_trait;
use maybe_sync::Mutex;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::sync::{Arc, RwLock};
use web_time::{Duration, SystemTime};

use super::Layer;

pub struct RasterTileLayer {
    tile_provider: Arc<dyn TileProvider<RasterTile>>,
    tile_scheme: TileScheme,
    tile_renders: RwLock<HashMap<TileIndex, RenderedTile>>,
    prev_drawn_tiles: Mutex<Vec<TileIndex>>,
    fade_in_duration: Duration,
}

struct RenderedTile {
    bundle: Box<dyn PackedBundle>,
    first_drawn: SystemTime,
    is_opaque: bool,
    primitive_id: PrimitiveId,
}

impl RasterTileLayer {
    pub fn from_url(
        tile_source: impl TileSource + 'static,
        tile_scheme: TileScheme,
        messenger: Option<Box<dyn Messenger>>,
    ) -> Self {
        let tile_provider = UrlTileProvider::new(Box::new(tile_source), messenger);
        Self {
            tile_provider: Arc::new(tile_provider),
            tile_scheme,
            tile_renders: RwLock::new(HashMap::new()),
            prev_drawn_tiles: Mutex::new(vec![]),
            fade_in_duration: Duration::from_millis(300),
        }
    }

    fn get_tiles_to_draw(&self, view: &MapView) -> Vec<RasterTile> {
        let mut tiles = vec![];
        let Some(tile_iter) = self.tile_scheme.iter_tiles(view) else {
            return vec![];
        };

        let tile_renders = self.tile_renders.read().unwrap();
        let mut to_substitute = vec![];
        for index in tile_iter {
            match self.tile_provider.get_tile(index) {
                None => to_substitute.push(index),
                Some(v) => {
                    tiles.push((index, v));
                    if let Some(tile) = tile_renders.get(&index) {
                        if !tile.is_opaque {
                            to_substitute.push(index);
                        }
                    } else {
                        to_substitute.push(index);
                    }
                }
            }
        }

        let prev_drawn = self.prev_drawn_tiles.lock();
        let mut substitute_indices = HashSet::new();
        for index in to_substitute {
            let mut next_level = index;
            let mut substituted = false;

            while let Some(subst) = self.tile_scheme.get_substitutes(next_level) {
                let mut need_more = false;
                for substitute_index in subst {
                    // todo: this will not work correctly if a tile is substituted by more then 1 tile
                    next_level = substitute_index;

                    if let Some(tile) = self.tile_provider.get_tile(substitute_index) {
                        if !substitute_indices.contains(&substitute_index) {
                            tiles.push((substitute_index, tile));
                            substitute_indices.insert(substitute_index);
                        }

                        if let Some(rendered) = tile_renders.get(&substitute_index) {
                            if !rendered.is_opaque {
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
                let required_bbox = self.tile_scheme.tile_bbox(index).unwrap();
                for prev in prev_drawn.iter() {
                    if !substitute_indices.contains(prev)
                        && self
                            .tile_scheme
                            .tile_bbox(*prev)
                            .unwrap()
                            .intersects(required_bbox)
                    {
                        substitute_indices.insert(*prev);
                        tiles.push((*prev, self.tile_provider.get_tile(*prev).unwrap()));
                    }
                }
            }
        }

        tiles.sort_unstable_by(|(index_a, _), (index_b, _)| index_a.z.cmp(&index_b.z));
        tiles.into_iter().map(|(_, tile)| tile).collect()
    }

    fn prepare_tile_renders(&self, tiles: &[RasterTile], canvas: &mut dyn Canvas) {
        let mut tile_renders = self.tile_renders.write().unwrap();
        let mut requires_redraw = false;

        let now = SystemTime::now();
        for tile in tiles {
            if tile_renders
                .get(&tile.index)
                .and_then(|t| (!t.is_opaque).then_some(()))
                .is_some()
            {
                let render = tile_renders.remove(&tile.index).unwrap();

                let RenderedTile {
                    bundle,
                    first_drawn,
                    primitive_id,
                    ..
                } = render;
                let since_drawn = now
                    .duration_since(first_drawn)
                    .unwrap_or(Duration::from_millis(0));
                let opacity = ((since_drawn.as_secs_f64() / self.fade_in_duration.as_secs_f64())
                    .min(1.0)
                    * 255.0) as u8;
                let is_opaque = opacity == 255;
                if !is_opaque {
                    requires_redraw = true;
                }

                let mut unpacked = bundle.unpack();
                unpacked.modify_image(primitive_id, ImagePaint { opacity });
                let packed = canvas.pack_unpacked(unpacked);

                tile_renders.insert(
                    tile.index,
                    RenderedTile {
                        bundle: packed,
                        first_drawn,
                        is_opaque,
                        primitive_id,
                    },
                );
            }

            if tile_renders.get(&tile.index).is_none() {
                let mut bundle = canvas.create_bundle();

                // todo: there should not be clone() here
                let id = bundle.add_image(
                    (*tile.decoded_image).clone(),
                    self.tile_scheme
                        .tile_bbox(tile.index)
                        .unwrap()
                        .into_quadrangle(),
                    ImagePaint { opacity: 0 },
                );
                let packed = canvas.pack_bundle(bundle);
                tile_renders.insert(
                    tile.index,
                    RenderedTile {
                        bundle: packed,
                        first_drawn: now,
                        is_opaque: false,
                        primitive_id: id,
                    },
                );

                requires_redraw = true;
            }
        }

        if requires_redraw {
            self.tile_provider.request_redraw();
        }
    }
}

#[async_trait]
impl Layer for RasterTileLayer {
    fn render(&self, view: &MapView, canvas: &mut dyn Canvas) {
        let tiles = self.get_tiles_to_draw(view);
        self.prepare_tile_renders(&tiles, canvas);

        let tile_renders = self.tile_renders.read().unwrap();
        let mut to_draw = Vec::new();
        for tile in &tiles {
            if let Some(rendered) = tile_renders.get(&tile.index) {
                to_draw.push(&*rendered.bundle);
            }
        }

        canvas.draw_bundles(&to_draw);
        *self.prev_drawn_tiles.lock() = tiles.iter().map(|t| t.index).collect();
    }

    fn prepare(&self, view: &MapView, _renderer: &Arc<RwLock<dyn Renderer>>) {
        if let Some(iter) = self.tile_scheme.iter_tiles(view) {
            for index in iter {
                let tile_provider = self.tile_provider.clone();
                crate::async_runtime::spawn(async move { tile_provider.load_tile(index).await });
            }
        }
    }

    fn set_messenger(&self, messenger: Box<dyn Messenger>) {
        self.tile_provider.set_messenger(messenger);
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl TileProvider<RasterTile> for UrlTileProvider<RasterTile> {
    fn get_tile(&self, index: TileIndex) -> Option<RasterTile> {
        self.get_tile_int(index)
    }

    async fn load_tile(&self, index: TileIndex) -> Result<(), GalileoError> {
        if self.loaded_tiles.read().unwrap().contains_key(&index) {
            return Ok(());
        } else {
            self.loaded_tiles
                .write()
                .unwrap()
                .insert(index, TileState::Loading);
        }

        let url = (*self.url_source)(index);
        log::info!("Loading tile {index:?} from {url}");
        match self.platform_service.load_image_url(&url).await {
            Ok(image) => {
                let tile = RasterTile {
                    decoded_image: Arc::new(image),
                    index,
                };
                self.loaded_tiles
                    .write()
                    .unwrap()
                    .insert(index, TileState::Loaded(tile));

                if let Some(messenger) = &(*self.messenger.read().unwrap()) {
                    messenger.request_redraw();
                }

                Ok(())
            }
            Err(e) => {
                log::info!("Failed to load tile {index:?}");
                Err(e)
            }
        }
    }

    fn set_messenger(&self, messenger: Box<dyn Messenger>) {
        *self.messenger.write().unwrap() = Some(messenger);
    }

    fn request_redraw(&self) {
        if let Some(messenger) = &(*self.messenger.read().unwrap()) {
            messenger.request_redraw();
        }
    }
}

#[derive(Debug, Clone)]
pub struct RasterTile {
    pub decoded_image: Arc<DecodedImage>,
    pub index: TileIndex,
}
