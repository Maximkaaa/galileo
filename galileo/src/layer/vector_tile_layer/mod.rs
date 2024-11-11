//! [Vector tile layers](VectorTileLayer) load prepared vector tiles using a [data provider](VectorTileProviderT)
//! and draw them to the map with the given [`VectorTileStyle`].

use maybe_sync::{MaybeSend, MaybeSync};
use std::any::Any;
use std::collections::HashSet;
use std::sync::Arc;

use nalgebra::Point2;

use galileo_mvt::{MvtFeature, MvtGeometry};
use galileo_types::cartesian::CartesianPoint2d;
use galileo_types::geometry::CartesianGeometry2d;
pub use vector_tile::VectorTile;

use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::vector_tile_layer::tile_provider::loader::VectorTileLoader;
use crate::layer::vector_tile_layer::tile_provider::processor::VectorTileProcessor;
use crate::layer::vector_tile_layer::tile_provider::{VectorTileProvider, VtStyleId};
use crate::layer::Layer;
use crate::messenger::Messenger;
use crate::render::{Canvas, PackedBundle, RenderOptions};
use crate::tile_scheme::TileSchema;
use crate::view::MapView;

pub mod style;
pub mod tile_provider;
mod vector_tile;

/// Vector tile layers use [`Providers`](VectorTileProviderT) to load prepared vector tiles, and then render them using
/// specified [styles](VectorTileStyle).
pub struct VectorTileLayer<Loader, Processor>
where
    Loader: VectorTileLoader + MaybeSend + MaybeSync + 'static,
    Processor: VectorTileProcessor + MaybeSend + MaybeSync + 'static,
{
    tile_provider: VectorTileProvider<Loader, Processor>,
    tile_scheme: TileSchema,
    style_id: VtStyleId,
}

impl<Loader, Processor> Layer for VectorTileLayer<Loader, Processor>
where
    Loader: VectorTileLoader + MaybeSend + MaybeSync + 'static,
    Processor: VectorTileProcessor + MaybeSend + MaybeSync + 'static,
{
    fn render(&self, view: &MapView, canvas: &mut dyn Canvas) {
        let tiles = self.get_tiles_to_draw(view, canvas);
        let to_render: Vec<&dyn PackedBundle> = tiles.iter().map(|v| &**v).collect();

        canvas.draw_bundles(&to_render, RenderOptions::default());
    }

    fn prepare(&self, view: &MapView) {
        if let Some(iter) = self.tile_scheme.iter_tiles(view) {
            for index in iter {
                self.tile_provider.load_tile(index, self.style_id);
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

impl<Loader, Processor> VectorTileLayer<Loader, Processor>
where
    Loader: VectorTileLoader + MaybeSend + MaybeSync + 'static,
    Processor: VectorTileProcessor + MaybeSend + MaybeSync + 'static,
{
    /// Style of the layer.
    pub fn style(&self) -> Arc<VectorTileStyle> {
        self.tile_provider
            .get_style(self.style_id)
            .unwrap_or_default()
    }

    /// Creates a new layer with the given url source.
    pub async fn from_url(
        mut tile_provider: VectorTileProvider<Loader, Processor>,
        style: VectorTileStyle,
        tile_scheme: TileSchema,
    ) -> Self {
        let style_id = tile_provider.add_style(style).await;
        Self {
            tile_provider,
            tile_scheme,
            style_id,
        }
    }

    fn get_tiles_to_draw(&self, view: &MapView, canvas: &dyn Canvas) -> Vec<Arc<dyn PackedBundle>> {
        let mut tiles = vec![];
        let Some(tile_iter) = self.tile_scheme.iter_tiles(view) else {
            return vec![];
        };

        let indices: Vec<_> = tile_iter.collect();
        self.tile_provider
            .pack_tiles(&indices, self.style_id, canvas);

        let mut to_substitute = vec![];
        for index in &indices {
            match self.tile_provider.get_tile(*index, self.style_id) {
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

                if let Some(tile) = self.tile_provider.get_tile(substitute_index, self.style_id) {
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

    /// Change style of the layer and redraw it.
    pub async fn update_style(&mut self, style: VectorTileStyle) {
        let new_style_id = self.tile_provider.add_style(style).await;
        self.style_id = new_style_id;
    }

    /// Returns features, visible in the layer at the given point with the given map view.
    pub fn get_features_at(
        &self,
        point: &impl CartesianPoint2d<Num = f64>,
        view: &MapView,
    ) -> Vec<(String, MvtFeature)> {
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

                if let Some(mvt_tile) = self.tile_provider.get_mvt_tile(index) {
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
