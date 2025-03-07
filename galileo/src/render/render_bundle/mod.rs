//! [`RenderBundle`] is used to store primitives and prepare them for rendering with the rendering backend.

use galileo_types::cartesian::{CartesianPoint3d, Point2, Vector2};
use galileo_types::contour::Contour;
use galileo_types::Polygon;
use num_traits::AsPrimitive;
use screen_set::ScreenRenderSet;

use super::text::TextStyle;
use crate::decoded_image::DecodedImage;
use crate::render::point_paint::PointPaint;
use crate::render::render_bundle::world_set::WorldRenderSet;
use crate::render::{ImagePaint, LinePaint, PolygonPaint};

pub(crate) mod screen_set;
pub(crate) mod world_set;

/// Render bundle is used to store render primitives and prepare them to be rendered with the rendering backend.
#[derive(Debug, Default, Clone)]
pub struct RenderBundle {
    pub(crate) world_set: WorldRenderSet,
    pub(crate) screen_sets: Vec<ScreenRenderSet>,
}

impl RenderBundle {
    /// Adds an image to the bundle.
    pub fn add_image(&mut self, image: DecodedImage, vertices: [Point2; 4], paint: ImagePaint) {
        self.world_set.add_image(image, vertices, paint);
    }

    /// Adds a point to the bundle.
    pub fn add_point<N, P>(&mut self, point: &P, paint: &PointPaint, _min_resolution: f64)
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
    {
        self.world_set.add_point(point, paint);
    }

    /// Adds a line to the bundle.
    pub fn add_line<N, P, C>(&mut self, line: &C, paint: &LinePaint, min_resolution: f64)
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
        C: Contour<Point = P>,
    {
        self.world_set.add_line(line, paint, min_resolution);
    }

    /// Adds a polygon to the bundle.
    pub fn add_polygon<N, P, Poly>(
        &mut self,
        polygon: &Poly,
        paint: &PolygonPaint,
        min_resolution: f64,
    ) where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
        Poly: Polygon,
        Poly::Contour: Contour<Point = P>,
    {
        self.world_set.add_polygon(polygon, paint, min_resolution);
    }

    /// Adds a label to the bundle.
    pub fn add_label<N, P>(
        &mut self,
        position: &P,
        text: &str,
        style: &TextStyle,
        offset: Vector2<f32>,
    ) where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
    {
        if let Some(set) = ScreenRenderSet::new_from_label(position, text, style, offset) {
            self.screen_sets.push(set);
        }
    }
}
