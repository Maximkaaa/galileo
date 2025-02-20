//! [`RenderBundle`] is used to store primitives and prepare them for rendering with the rendering backend.

use galileo_types::cartesian::{CartesianPoint3d, Point2d};
use galileo_types::contour::Contour;
use galileo_types::Polygon;
use num_traits::AsPrimitive;

use crate::decoded_image::DecodedImage;
use crate::render::point_paint::PointPaint;
use crate::render::render_bundle::tessellating::TessellatingRenderBundle;
use crate::render::{ImagePaint, LinePaint, PolygonPaint};

pub(crate) mod tessellating;

/// Render bundle is used to store render primitives and prepare them to be rendered with the rendering backend.
#[derive(Debug, Clone)]
pub struct RenderBundle(pub(crate) RenderBundleType);

#[derive(Debug, Clone)]
pub(crate) enum RenderBundleType {
    Tessellating(TessellatingRenderBundle),
}

impl RenderBundle {
    /// Returns approximate amount of memory used by this bundle.
    pub fn approx_buffer_size(&self) -> usize {
        match &self.0 {
            RenderBundleType::Tessellating(inner) => inner.approx_buffer_size(),
        }
    }

    /// Sets the value for `approx_buffer_size`.
    ///
    /// This can be useful for better memory management when used buffers size cannot be calculated
    /// properly.
    ///
    /// Note, that consequent changes to the bundle will change the given value as if it was the
    /// calculated one.
    pub fn set_approx_buffer_size(&mut self, size: usize) {
        match &mut self.0 {
            RenderBundleType::Tessellating(inner) => inner.set_approx_buffer_size(size),
        }
    }

    /// Set the clip area for drawing. Only primitives inside the clipped area will be displayed after rendering.
    pub fn clip_area<N, P, Poly>(&mut self, polygon: &Poly)
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
        Poly: Polygon,
        Poly::Contour: Contour<Point = P>,
    {
        match &mut self.0 {
            RenderBundleType::Tessellating(inner) => inner.clip_area(polygon),
        }
    }

    /// Adds an image to the bundle.
    pub fn add_image(&mut self, image: DecodedImage, vertices: [Point2d; 4], paint: ImagePaint) {
        match &mut self.0 {
            RenderBundleType::Tessellating(inner) => inner.add_image(image, vertices, paint),
        }
    }

    /// Adds a point to the bundle.
    pub fn add_point<N, P>(&mut self, point: &P, paint: &PointPaint, _min_resolution: f64)
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
    {
        match &mut self.0 {
            RenderBundleType::Tessellating(inner) => inner.add_point(point, paint),
        }
    }

    /// Adds a line to the bundle.
    pub fn add_line<N, P, C>(&mut self, line: &C, paint: &LinePaint, min_resolution: f64)
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
        C: Contour<Point = P>,
    {
        match &mut self.0 {
            RenderBundleType::Tessellating(inner) => inner.add_line(line, paint, min_resolution),
        }
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
        match &mut self.0 {
            RenderBundleType::Tessellating(inner) => {
                inner.add_polygon(polygon, paint, min_resolution)
            }
        }
    }
}
