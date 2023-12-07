use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::sync::{Arc, RwLock};

use crate::bounding_box::BoundingBox;
use async_trait::async_trait;
use maybe_sync::{MaybeSend, MaybeSync};

use crate::error::GalileoError;
use crate::layer::tile_provider::{TileProvider, TileSource, TileState, UrlTileProvider};
use crate::messenger::Messenger;
use crate::platform::PlatformService;
use crate::primitives::{DecodedImage, Image, Size};
use crate::render::{Canvas, Renderer};
use crate::tile_scheme::{TileIndex, TileScheme};
use crate::view::MapView;
use crate::winit::WinitMessenger;

use super::Layer;

pub struct RasterTileLayer {
    tile_provider: Arc<dyn TileProvider<RasterTile>>,
    tile_scheme: TileScheme,
    tile_renders: RwLock<HashMap<TileIndex, Box<dyn Image>>>,
}

impl RasterTileLayer {
    pub fn from_url(tile_source: impl TileSource + 'static, messenger: WinitMessenger) -> Self {
        let tile_provider = UrlTileProvider::new(Box::new(tile_source), messenger);
        Self {
            tile_provider: Arc::new(tile_provider),
            tile_scheme: TileScheme::web(18),
            tile_renders: RwLock::new(HashMap::new()),
        }
    }

    fn get_tiles_to_draw<'a>(&self, resolution: f64, bbox: BoundingBox) -> Vec<RasterTile> {
        let mut tiles = vec![];
        let Some(tile_iter) = self.tile_scheme.iter_tiles(resolution, bbox) else {
            return vec![];
        };

        let mut to_substitute = vec![];
        for index in tile_iter {
            match self.tile_provider.get_tile(index) {
                None => to_substitute.push(index),
                Some(v) => tiles.push((index, v)),
            }
        }

        let mut substitute_indices = HashSet::new();
        for index in to_substitute {
            let tile_bbox = self
                .tile_scheme
                .tile_bbox(index)
                .unwrap()
                .shrink(resolution);
            if index.z == 0 {
                continue;
            }

            'indexer: for z in (0..index.z).rev() {
                if let Some(curr_resolution) = self.tile_scheme.lod_resolution(z) {
                    for substitute_index in self
                        .tile_scheme
                        .iter_tiles(curr_resolution, tile_bbox)
                        .unwrap()
                    {
                        if let Some(tile) = self.tile_provider.get_tile(substitute_index) {
                            if !substitute_indices.contains(&substitute_index) {
                                tiles.push((substitute_index, tile));
                                substitute_indices.insert(substitute_index);
                            }

                            break 'indexer;
                        }
                    }
                }
            }
        }

        tiles.sort_unstable_by(|(index_a, _), (index_b, _)| index_a.z.cmp(&index_b.z));
        tiles.into_iter().map(|(_, tile)| tile).collect()
    }
}

#[async_trait]
impl Layer for RasterTileLayer {
    fn render<'a>(&self, map_view: MapView, canvas: &'a mut dyn Canvas) {
        let bbox = map_view.get_bbox(canvas.size());
        let tiles = self.get_tiles_to_draw(map_view.resolution(), bbox);

        let tile_renders = self.tile_renders.try_read().unwrap();
        let mut renders_to_add: Vec<(TileIndex, Box<dyn Image>)> = Vec::new();
        let mut to_draw = Vec::new();
        for tile in &tiles {
            if let Some(image) = tile_renders.get(&tile.index) {
                to_draw.push(image);
            } else {
                let image = canvas.create_image(
                    &tile.decoded_image,
                    self.tile_scheme.tile_bbox(tile.index).unwrap(),
                );
                renders_to_add.push((tile.index, image));
            }
        }

        canvas.draw_images(&to_draw);
        if !renders_to_add.is_empty() {
            let to_add_refs = renders_to_add.iter().map(|(_, v)| v).collect();
            canvas.draw_images(&to_add_refs);
        }

        drop(tile_renders);

        if !renders_to_add.is_empty() {
            let mut tile_renders = self.tile_renders.try_write().unwrap();
            for (index, image) in renders_to_add {
                tile_renders.insert(index, image);
            }
        }
    }

    fn prepare(&self, map_view: MapView, map_size: Size, _renderer: &Arc<RwLock<dyn Renderer>>) {
        let bbox = map_view.get_bbox(map_size);
        if let Some(iter) = self.tile_scheme.iter_tiles(map_view.resolution(), bbox) {
            for index in iter {
                let tile_provider = self.tile_provider.clone();
                crate::async_runtime::spawn(async move { tile_provider.load_tile(index).await });
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        todo!()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<M: Messenger + MaybeSend + MaybeSync> TileProvider<RasterTile>
    for UrlTileProvider<M, RasterTile>
{
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

                self.messenger.request_redraw();
                Ok(())
            }
            Err(e) => {
                log::info!("Failed to load tile {index:?}");
                Err(e)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct RasterTile {
    pub decoded_image: Arc<DecodedImage>,
    pub index: TileIndex,
}
