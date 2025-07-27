//! [Vector tile layers](VectorTileLayer) load prepared vector tiles using a
//! [data provider](crate::layer::vector_tile_layer::tile_provider::VectorTileProvider) and draw them to the map with
//! the given [`VectorTileStyle`].

use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

use galileo_mvt::{MvtFeature, MvtGeometry};
use galileo_types::cartesian::{CartesianPoint2d, Point2, Point3};
use galileo_types::geometry::CartesianGeometry2d;
use galileo_types::impls::{ClosedContour, Polygon};
use galileo_types::MultiPolygon;
use parking_lot::Mutex;
pub use vector_tile::VectorTile;

use crate::layer::attribution::Attribution;
use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::vector_tile_layer::tile_provider::{VectorTileProvider, VtStyleId};
use crate::layer::Layer;
use crate::messenger::Messenger;
use crate::render::render_bundle::RenderBundle;
use crate::render::{Canvas, PackedBundle, PolygonPaint, RenderOptions};
use crate::tile_schema::TileSchema;
use crate::view::MapView;
use crate::Color;

mod builder;
pub mod style;
pub mod tile_provider;
mod vector_tile;
pub use builder::VectorTileLayerBuilder;

use super::tiles::TilesContainer;

/// Vector tile layers use [tile providers](VectorTileProvider) to load prepared vector tiles, and then render them using
/// specified [styles](VectorTileStyle).
pub struct VectorTileLayer {
    tile_provider: VectorTileProvider,
    tile_schema: TileSchema,
    style_id: VtStyleId,
    displayed_tiles: TilesContainer<VtStyleId, VectorTileProvider>,
    prev_background: Mutex<Option<PreviousBackground>>,
    attribution: Option<Attribution>,
}

impl std::fmt::Debug for VectorTileLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RasterTileLayer")
            .field("tile_schema", &self.tile_schema)
            .field("style_id", &self.style_id)
            .finish()
    }
}

#[derive(Debug, Copy, Clone)]
struct PreviousBackground {
    color: Color,
    replaced_at: web_time::Instant,
}

impl Layer for VectorTileLayer {
    fn render(&self, view: &MapView, canvas: &mut dyn Canvas) {
        self.update_displayed_tiles(view, canvas);

        let Some(background_bundle) = self.create_background_bundle(view, canvas) else {
            // View is impossible to render
            return;
        };

        let displayed_tiles = self.displayed_tiles.tiles.lock();
        let to_render: Vec<(&dyn PackedBundle, f32)> = std::iter::once((&*background_bundle, 1.0))
            .chain(displayed_tiles.iter().map(|v| (&*v.bundle, v.opacity)))
            .collect();

        canvas.draw_bundles_with_opacity(&to_render, RenderOptions::default());
    }

    fn prepare(&self, view: &MapView, _canvas: &mut dyn Canvas) {
        if let Some(iter) = self.tile_schema.iter_tiles(view) {
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

    fn tile_schema(&self) -> Option<TileSchema> {
        Some(self.tile_schema.clone())
    }

    fn attribution(&self) -> Option<Attribution> {
        self.attribution.clone()
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
    pub fn new(
        mut tile_provider: VectorTileProvider,
        style: VectorTileStyle,
        tile_schema: TileSchema,
        attribution: Option<Attribution>,
    ) -> Self {
        let style_id = tile_provider.add_style(style);
        Self {
            tile_provider: tile_provider.clone(),
            tile_schema: tile_schema.clone(),
            style_id,
            displayed_tiles: TilesContainer::new(tile_schema, tile_provider),
            prev_background: Default::default(),
            attribution,
        }
    }

    fn update_displayed_tiles(&self, view: &MapView, canvas: &dyn Canvas) {
        let Some(tile_iter) = self.tile_schema.iter_tiles(view) else {
            return;
        };

        let needed_indices: Vec<_> = tile_iter.collect();
        self.tile_provider
            .pack_tiles(&needed_indices, self.style_id, canvas);
        let requires_redraw = self
            .displayed_tiles
            .update_displayed_tiles(needed_indices, self.style_id);

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
        if let Some(curr_style) = self.tile_provider.get_style(self.style_id) {
            *self.prev_background.lock() = Some(PreviousBackground {
                color: curr_style.background,
                replaced_at: web_time::Instant::now(),
            });
        }
        self.tile_provider.drop_style(self.style_id);
        self.style_id = new_style_id;
    }

    /// Returns features, visible in the layer at the given point with the given map view.
    pub fn get_features_at(
        &self,
        point: &impl CartesianPoint2d<Num = f64>,
        view: &MapView,
    ) -> Vec<(String, MvtFeature)> {
        const PIXEL_TOLERANCE: f64 = 2.0;
        let view_resolution = view.resolution();
        let res_tolerance = view_resolution * PIXEL_TOLERANCE;

        let mut features = vec![];
        if let Some(iter) = self.tile_schema.iter_tiles(view) {
            for index in iter {
                let Some(tile_bbox) = self.tile_schema.tile_bbox(index) else {
                    continue;
                };

                if !tile_bbox.shrink(-res_tolerance).contains(point) {
                    continue;
                }

                let Some(lod_resolution) = self.tile_schema.lod_resolution(index.z) else {
                    continue;
                };

                let tile_resolution = lod_resolution * self.tile_schema.tile_width() as f64;

                let tile_point = Point2::new(
                    ((point.x() - tile_bbox.x_min()) / tile_resolution) as f32,
                    ((tile_bbox.y_max() - point.y()) / tile_resolution) as f32,
                );

                let tolerance = ((view.resolution() / tile_resolution) * PIXEL_TOLERANCE) as f32;

                if let Some(mvt_tile) = self.tile_provider.get_mvt_tile(index) {
                    for layer in &mvt_tile.layers {
                        for feature in &layer.features {
                            match &feature.geometry {
                                MvtGeometry::Point(points) => {
                                    if points
                                        .iter()
                                        .any(|p| p.is_point_inside(&tile_point, tolerance))
                                    {
                                        features.push((layer.name.clone(), feature.clone()));
                                    }
                                }
                                MvtGeometry::LineString(contours) => {
                                    if contours.is_point_inside(&tile_point, tolerance) {
                                        features.push((layer.name.clone(), feature.clone()));
                                    }
                                }
                                MvtGeometry::Polygon(polygons) => {
                                    if polygons
                                        .polygons()
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

    fn create_background_bundle(
        &self,
        view: &MapView,
        canvas: &mut dyn Canvas,
    ) -> Option<Box<dyn PackedBundle>> {
        let mut bundle = RenderBundle::new(canvas.dpi_scale_factor());
        let bbox = view.get_bbox()?;
        let bounds = Polygon::new(
            ClosedContour::new(vec![
                Point3::new(bbox.x_min(), bbox.y_min(), 0.0),
                Point3::new(bbox.x_min(), bbox.y_max(), 0.0),
                Point3::new(bbox.x_max(), bbox.y_max(), 0.0),
                Point3::new(bbox.x_max(), bbox.y_min(), 0.0),
            ]),
            vec![],
        );
        let style = self.tile_provider.get_style(self.style_id)?;

        let mut prev_background = self.prev_background.lock();
        let color = match *prev_background {
            Some(prev) => {
                let k = web_time::Instant::now()
                    .duration_since(prev.replaced_at)
                    .as_secs_f32()
                    / self.fade_in_time().as_secs_f32();

                if k >= 1.0 {
                    *prev_background = None;
                    style.background
                } else {
                    prev.color.blend(
                        style
                            .background
                            .with_alpha((style.background.a() as f32 * k) as u8),
                    )
                }
            }
            None => style.background,
        };

        bundle.add_polygon(&bounds, &PolygonPaint { color }, view.resolution());

        Some(canvas.pack_bundle(&bundle))
    }

    /// Returns the reference to the layer's tile provider.
    pub fn provider(&self) -> &VectorTileProvider {
        &self.tile_provider
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::native::vt_processor::ThreadVtProcessor;
    use crate::tests::TestTileLoader;

    fn test_layer() -> VectorTileLayer {
        let tile_schema = TileSchema::web(18);
        let mut provider = VectorTileProvider::new(
            Arc::new(TestTileLoader {}),
            Arc::new(ThreadVtProcessor::new(tile_schema.clone())),
        );

        let style_id = provider.add_style(VectorTileStyle::default());
        VectorTileLayer {
            tile_provider: provider.clone(),
            tile_schema: TileSchema::web(18),
            style_id,
            displayed_tiles: TilesContainer::new(tile_schema, provider),
            prev_background: Default::default(),
            attribution: None,
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
