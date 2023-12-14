use crate::layer::tile_provider::TileSource;
use crate::layer::Layer;
use crate::messenger::Messenger;
use crate::render::{Canvas, PackedBundle, Renderer};
use crate::tile_scheme::TileScheme;
use crate::view::MapView;
use std::any::Any;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use crate::layer::vector_tile::style::VectorTileStyle;
use crate::layer::vector_tile::tile_provider::{LockedTileStore, VectorTileProvider};
use crate::layer::vector_tile::vector_tile::VectorTile;
use galileo_mvt::{MvtFeature, MvtGeometry};
use galileo_types::bounding_rect::BoundingRect;
use galileo_types::geometry::Geometry;
use galileo_types::{CartesianPoint2d, Point2};

pub mod style;
pub mod tile_provider;
pub mod vector_tile;

pub struct VectorTileLayer<Provider: VectorTileProvider> {
    tile_provider: Provider,
    tile_scheme: TileScheme,
    style: VectorTileStyle,
}

impl<Provider: VectorTileProvider + 'static> Layer for VectorTileLayer<Provider> {
    fn render<'a>(&self, map_view: MapView, canvas: &'a mut dyn Canvas) {
        let bbox = map_view.get_bbox();

        let tiles_store = self.tile_provider.read();
        let tiles = self.get_tiles_to_draw(map_view.resolution(), bbox, &tiles_store);
        let to_render: Vec<&Box<dyn PackedBundle>> = tiles.iter().map(|v| &v.bundle).collect();

        canvas.draw_bundles(&to_render);
    }

    fn prepare(&self, view: MapView, renderer: &Arc<RwLock<dyn Renderer>>) {
        let bbox = view.get_bbox();
        if let Some(iter) = self.tile_scheme.iter_tiles(view.resolution(), bbox) {
            for index in iter {
                if self.tile_provider.supports(&**renderer) {
                    self.tile_provider.load_tile(index, &self.style, renderer);
                }
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl<Provider: VectorTileProvider> VectorTileLayer<Provider> {
    pub fn style(&self) -> &VectorTileStyle {
        &self.style
    }

    pub fn from_url(
        tile_source: impl TileSource + 'static,
        style: VectorTileStyle,
        messenger: impl Messenger + 'static,
        tile_scheme: TileScheme,
    ) -> Self {
        let tile_provider = Provider::create(messenger, tile_source, tile_scheme.clone());

        Self {
            tile_provider,
            tile_scheme,
            style,
        }
    }

    fn get_tiles_to_draw<'a>(
        &self,
        resolution: f64,
        bbox: BoundingRect,
        tiles_store: &'a LockedTileStore,
    ) -> Vec<&'a VectorTile> {
        let mut tiles = vec![];
        let Some(tile_iter) = self.tile_scheme.iter_tiles(resolution, bbox) else {
            return vec![];
        };

        let mut to_substitute = vec![];
        for index in tile_iter {
            match tiles_store.get_tile(index) {
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
                        if let Some(tile) = tiles_store.get_tile(substitute_index) {
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

    pub fn update_style(&mut self, style: VectorTileStyle) {
        self.style = style;
        self.tile_provider.update_style();
    }

    pub fn get_features_at(
        &self,
        point: &impl CartesianPoint2d<Num = f64>,
        resolution: f64,
    ) -> Vec<(String, MvtFeature)> {
        let bbox = BoundingRect::new(point.x(), point.y(), point.x(), point.y());

        let tile_store = self.tile_provider.read();
        let mut features = vec![];
        if let Some(iter) = self.tile_scheme.iter_tiles(resolution, bbox) {
            for index in iter {
                let tile_bbox = self.tile_scheme.tile_bbox(index).unwrap();
                let lod_resolution = self.tile_scheme.lod_resolution(index.z).unwrap();
                let tile_resolution = lod_resolution * self.tile_scheme.tile_width() as f64;

                let tile_point = Point2::new(
                    ((point.x() - tile_bbox.x_min()) / tile_resolution) as f32,
                    ((tile_bbox.y_max() - point.y()) / tile_resolution) as f32,
                );

                let tolerance = (resolution / tile_resolution) as f32 * 2.0;

                if let Some(tile) = tile_store.get_tile(index) {
                    for layer in &tile.mvt_tile.layers {
                        for feature in &layer.features {
                            match &feature.geometry {
                                MvtGeometry::Point(_) => {}
                                MvtGeometry::LineString(contours) => {
                                    if contours
                                        .iter()
                                        .any(|c| c.is_point_inside(&tile_point, tolerance))
                                    {
                                        features.push((layer.name.clone(), feature.clone()));
                                    }
                                }
                                MvtGeometry::Polygon(polygons) => {
                                    if polygons
                                        .iter()
                                        .any(|p| p.is_point_inside(&tile_point, tolerance))
                                    {
                                        features.push((layer.name.clone(), feature.clone()));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        features
    }
}
