//! [`RenderBundle`] is used to store primitives and prepare them for rendering with the rendering backend.

use galileo_types::cartesian::{CartesianPoint3d, Point2d};
use galileo_types::contour::Contour;
use galileo_types::Polygon;
use num_traits::AsPrimitive;

use crate::decoded_image::DecodedImage;
use crate::render::point_paint::PointPaint;
use crate::render::render_bundle::tessellating::WorldRenderSet;
use crate::render::{ImagePaint, LinePaint, PolygonPaint};

pub(crate) mod tessellating;

/// Render bundle is used to store render primitives and prepare them to be rendered with the rendering backend.
#[derive(Debug, Default, Clone)]
pub struct RenderBundle {
    pub(crate) world_set: WorldRenderSet,
}

impl RenderBundle {
    /// Adds an image to the bundle.
    pub fn add_image(&mut self, image: DecodedImage, vertices: [Point2d; 4], paint: ImagePaint) {
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
}
