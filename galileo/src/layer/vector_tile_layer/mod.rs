use crate::layer::Layer;
use crate::messenger::Messenger;
use crate::render::{Canvas, PackedBundle, RenderOptions};
use crate::tile_scheme::TileSchema;
use crate::view::MapView;
use nalgebra::Point2;
use std::any::Any;
use std::collections::HashSet;

use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::vector_tile_layer::tile_provider::{LockedTileStore, VectorTileProvider};
use crate::layer::vector_tile_layer::vector_tile::VectorTile;
use galileo_mvt::{MvtFeature, MvtGeometry};
use galileo_types::cartesian::traits::cartesian_point::CartesianPoint2d;
use galileo_types::geometry::CartesianGeometry2d;

pub mod style;
pub mod tile_provider;
pub mod vector_tile;

pub struct VectorTileLayer<Provider: VectorTileProvider> {
    tile_provider: Provider,
    tile_scheme: TileSchema,
    style: VectorTileStyle,
}

impl<Provider: VectorTileProvider + 'static> Layer for VectorTileLayer<Provider> {
    fn render(&self, view: &MapView, canvas: &mut dyn Canvas) {
        let mut tiles_store = self.tile_provider.read();
        let tiles = self.get_tiles_to_draw(view, &mut tiles_store, canvas);
        let to_render: Vec<&dyn PackedBundle> = tiles.iter().map(|v| &*v.bundle).collect();

        canvas.draw_bundles(&to_render, RenderOptions::default());
    }

    fn prepare(&self, view: &MapView) {
        if let Some(iter) = self.tile_scheme.iter_tiles(view) {
            for index in iter {
                self.tile_provider.load_tile(index, &self.style);
            }
        }
    }

    fn set_messenger(&mut self, messenger: Box<dyn Messenger>) {
        self.tile_provider.set_messenger(messenger);
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
        tile_provider: Provider,
        style: VectorTileStyle,
        tile_scheme: TileSchema,
    ) -> Self {
        Self {
            tile_provider,
            tile_scheme,
            style,
        }
    }

    fn get_tiles_to_draw<'a>(
        &self,
        view: &MapView,
        tiles_store: &'a mut LockedTileStore,
        canvas: &dyn Canvas,
    ) -> Vec<&'a VectorTile> {
        let mut tiles = vec![];
        let Some(tile_iter) = self.tile_scheme.iter_tiles(view) else {
            return vec![];
        };

        let indices: Vec<_> = tile_iter.collect();

        for index in &indices {
            tiles_store.pack(*index, canvas);
        }

        let mut to_substitute = vec![];
        for index in &indices {
            match tiles_store.get_tile(*index) {
                None => to_substitute.push(*index),
                Some(v) => tiles.push((*index, v)),
            }
        }

        let mut substitute_indices = HashSet::new();
        for index in to_substitute {
            let mut substitute_index = index;
            while let Some(mut subst) = self.tile_scheme.get_substitutes(substitute_index) {
                substitute_index = match subst.next() {
                    Some(v) => v,
                    None => break,
                };

                if let Some(tile) = tiles_store.get_tile(substitute_index) {
                    if !substitute_indices.contains(&substitute_index) {
                        tiles.push((substitute_index, tile));
                        substitute_indices.insert(substitute_index);
                    }

                    break;
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
        view: &MapView,
    ) -> Vec<(String, MvtFeature)> {
        let tile_store = self.tile_provider.read();
        let mut features = vec![];
        if let Some(iter) = self.tile_scheme.iter_tiles(view) {
            for index in iter {
                let Some(tile_bbox) = self.tile_scheme.tile_bbox(index) else {
                    continue;
                };
                let Some(lod_resolution) = self.tile_scheme.lod_resolution(index.z) else {
                    continue;
                };

                let tile_resolution = lod_resolution * self.tile_scheme.tile_width() as f64;

                let tile_point = Point2::new(
                    ((point.x() - tile_bbox.x_min()) / tile_resolution) as f32,
                    ((tile_bbox.y_max() - point.y()) / tile_resolution) as f32,
                );

                let tolerance = (view.resolution() / tile_resolution) as f32 * 2.0;

                if let Some(mvt_tile) = tile_store.get_mvt_tile(index) {
                    for layer in &mvt_tile.layers {
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
