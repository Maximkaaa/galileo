//! [Vector tile layers](VectorTileLayer) load prepared vector tiles using a [data provider](VectorTileProviderT)
//! and draw them to the map with the given [`VectorTileStyle`].

use std::any::Any;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use nalgebra::Point2;

use galileo_mvt::{MvtFeature, MvtGeometry};
use galileo_types::cartesian::CartesianPoint2d;
use galileo_types::geometry::CartesianGeometry2d;
pub use vector_tile::VectorTile;

use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::vector_tile_layer::tile_provider::{VectorTileProvider, VtStyleId};
use crate::layer::Layer;
use crate::messenger::Messenger;
use crate::render::{Canvas, PackedBundle, RenderOptions};
use crate::tile_scheme::{TileIndex, TileSchema};
use crate::view::MapView;

pub mod style;
pub mod tile_provider;
mod vector_tile;

/// Vector tile layers use [`Providers`](VectorTileProviderT) to load prepared vector tiles, and then render them using
/// specified [styles](VectorTileStyle).
pub struct VectorTileLayer {
    tile_provider: VectorTileProvider,
    tile_scheme: TileSchema,
    style_id: VtStyleId,
    displayed_tiles: Mutex<Vec<DisplayedTile>>,
}

struct DisplayedTile {
    index: TileIndex,
    bundle: Arc<dyn PackedBundle>,
    style_id: VtStyleId,
    opacity: f32,
    displayed_at: web_time::Instant,
}

impl DisplayedTile {
    fn is_opaque(&self) -> bool {
        self.opacity >= 0.999
    }
}

impl Layer for VectorTileLayer {
    fn render(&self, view: &MapView, canvas: &mut dyn Canvas) {
        self.update_displayed_tiles(view, canvas);
        let displayed_tiles = self.displayed_tiles.lock().expect("mutex is poisoned");
        let to_render: Vec<(&dyn PackedBundle, f32)> = displayed_tiles
            .iter()
            .map(|v| (&*v.bundle, v.opacity))
            .collect();

        canvas.draw_bundles_with_opacity(&to_render, RenderOptions::default());
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

impl VectorTileLayer {
    /// Style of the layer.
    pub fn style(&self) -> Arc<VectorTileStyle> {
        self.tile_provider
            .get_style(self.style_id)
            .unwrap_or_default()
    }

    /// Creates a new layer with the given url source.
    pub fn from_url(
        mut tile_provider: VectorTileProvider,
        style: VectorTileStyle,
        tile_scheme: TileSchema,
    ) -> Self {
        let style_id = tile_provider.add_style(style);
        Self {
            tile_provider,
            tile_scheme,
            style_id,
            displayed_tiles: Default::default(),
        }
    }

    fn update_displayed_tiles(&self, view: &MapView, canvas: &dyn Canvas) {
        let mut tiles = vec![];
        let Some(tile_iter) = self.tile_scheme.iter_tiles(view) else {
            return;
        };

        let indices: Vec<_> = tile_iter.collect();
        self.tile_provider
            .pack_tiles(&indices, self.style_id, canvas);

        let mut displayed_tiles = self.displayed_tiles.lock().expect("mutex is poisoned");

        let mut to_substitute = vec![];
        for index in &indices {
            match self.tile_provider.get_tile(*index, self.style_id) {
                None => to_substitute.push(*index),
                Some(v) => {
                    tiles.push((*index, v));

                    if let Some(displayed) = displayed_tiles
                        .iter()
                        .find(|displayed| displayed.index == *index)
                    {
                        if !displayed.is_opaque() {
                            to_substitute.push(*index);
                        }
                    } else {
                        to_substitute.push(*index);
                    }
                }
            }
        }

        let mut substitute_indices = HashSet::new();
        for index in to_substitute {
            let mut substitute_index = index;
            if let Some(displayed) = displayed_tiles
                .iter()
                .find(|entry| entry.index == index && entry.style_id != self.style_id)
            {
                tiles.push((index, displayed.bundle.clone()));
                substitute_indices.insert(index);
                break;
            }

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

        let mut requires_redraw = false;
        let mut new_displayed = Vec::with_capacity(tiles.len());
        let now = web_time::Instant::now();
        let fade_in_time = self.fade_in_time();
        while let Some(mut displayed) = displayed_tiles.pop() {
            if tiles.iter().any(|(index, _)| *index == displayed.index) {
                if !displayed.is_opaque() {
                    requires_redraw = true;
                    displayed.opacity = ((now.duration_since(displayed.displayed_at)).as_secs_f64()
                        / fade_in_time.as_secs_f64())
                    .min(1.0) as f32;
                }

                new_displayed.push(displayed);
            }
        }

        for (index, bundle) in tiles {
            if !new_displayed
                .iter()
                .any(|v| v.index == index && v.style_id == self.style_id)
            {
                // Adding new tiles to the displayed list
                new_displayed.push(DisplayedTile {
                    index,
                    bundle,
                    style_id: self.style_id,
                    opacity: 0.0,
                    displayed_at: web_time::Instant::now(),
                });
                requires_redraw = true;
            }
        }

        new_displayed.sort_unstable_by(|a, b| a.index.z.cmp(&b.index.z));
        *displayed_tiles = new_displayed;

        if requires_redraw {
            self.tile_provider.request_redraw();
        }
    }

    fn fade_in_time(&self) -> Duration {
        Duration::from_millis(300)
    }

    /// Change style of the layer and redraw it.
    pub fn update_style(&mut self, style: VectorTileStyle) {
        let new_style_id = self.tile_provider.add_style(style);
        self.tile_provider.drop_style(self.style_id);
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

#[cfg(test)]
mod tests {
    use crate::{
        platform::native::vt_processor::ThreadVtProcessor,
        render::render_bundle::{
            tessellating::TessellatingRenderBundle, RenderBundle, RenderBundleType,
        },
        tests::TestTileLoader,
    };

    use super::*;

    fn test_layer() -> VectorTileLayer {
        let tile_schema = TileSchema::web(18);
        let empty_bundle = RenderBundle(RenderBundleType::Tessellating(
            TessellatingRenderBundle::new(),
        ));
        let mut provider = VectorTileProvider::new(
            Arc::new(TestTileLoader {}),
            Arc::new(ThreadVtProcessor::new(tile_schema.clone(), empty_bundle)),
        );

        let style_id = provider.add_style(VectorTileStyle::default());
        VectorTileLayer {
            tile_provider: provider,
            tile_scheme: TileSchema::web(18),
            style_id,
        }
    }

    #[test]
    fn update_style_drops_previous_style() {
        let mut layer = test_layer();
        let style_id = layer.style_id;
        assert!(layer.tile_provider.get_style(style_id).is_some());

        layer.update_style(VectorTileStyle::default());
        let new_style_id = layer.style_id;
        assert!(layer.tile_provider.get_style(new_style_id).is_some());
        assert!(layer.tile_provider.get_style(style_id).is_none());
    }
}
