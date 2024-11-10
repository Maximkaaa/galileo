use crate::decoded_image::DecodedImage;
use crate::layer::feature_layer::symbol::Symbol;
use crate::render::point_paint::PointPaint;
use crate::render::render_bundle::RenderPrimitive;
use crate::Color;
use galileo_types::cartesian::CartesianPoint3d;
use galileo_types::geometry::Geom;
use galileo_types::MultiPoint;
use nalgebra::Vector2;
use num_traits::AsPrimitive;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use crate::error::GalileoError;
use galileo_types::impls::{Contour, Polygon};
#[cfg(not(target_arch = "wasm32"))]
use std::ops::Deref;

/// Renders a point as a circle of fixes size.
#[derive(Debug, Copy, Clone)]
pub struct CirclePointSymbol {
    /// Color of the circle.
    pub color: Color,
    /// Diameter of the circle.
    pub size: f64,
}

impl CirclePointSymbol {
    /// Create a new instance.
    pub fn new(color: Color, size: f64) -> Self {
        Self { color, size }
    }
}

impl<F> Symbol<F> for CirclePointSymbol {
    fn render<'a, N, P>(
        &self,
        _feature: &F,
        geometry: &'a Geom<P>,
        _min_resolution: f64,
    ) -> Vec<RenderPrimitive<'a, N, P, Contour<P>, Polygon<P>>>
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N> + Clone,
    {
        let paint = PointPaint::circle(self.color, self.size as f32);
        match geometry {
            Geom::Point(point) => vec![RenderPrimitive::new_point(point.clone(), paint)],
            Geom::MultiPoint(points) => points
                .iter_points()
                .map(|p| RenderPrimitive::new_point(p.clone(), paint.clone()))
                .collect(),
            _ => vec![],
        }
    }
}

/// Symbol that renders a point with an image. The image size is fixed on the screen and does not depend on map
/// resolution.
pub struct ImagePointSymbol {
    image: Arc<DecodedImage>,
    offset: Vector2<f32>,
    scale: f32,
}

impl ImagePointSymbol {
    /// Loads the image from the file system path.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_path(path: &str, offset: Vector2<f32>, scale: f32) -> Result<Self, GalileoError> {
        let image = image::io::Reader::open(path)?
            .decode()
            .map_err(|_| GalileoError::ImageDecode)?;

        Ok(Self {
            image: Arc::new(DecodedImage::from_raw(
                Vec::from(image.to_rgba8().deref()),
                image.width(),
                image.height(),
            )?),
            offset,
            scale,
        })
    }
}

impl<F> Symbol<F> for ImagePointSymbol {
    fn render<'a, N, P>(
        &self,
        _feature: &F,
        geometry: &'a Geom<P>,
        _min_resolution: f64,
    ) -> Vec<RenderPrimitive<'a, N, P, Contour<P>, Polygon<P>>>
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N> + Clone,
    {
        let paint = PointPaint::image(self.image.clone(), self.offset, self.scale);

        match geometry {
            Geom::Point(point) => vec![RenderPrimitive::new_point(point.clone(), paint)],
            Geom::MultiPoint(points) => points
                .iter_points()
                .map(|point| RenderPrimitive::new_point(point.clone(), paint.clone()))
                .collect(),
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_symbol_from_file() {
        let symbol = ImagePointSymbol::from_path(
            "examples/data/pin-yellow.png",
            Vector2::new(0.5, 1.0),
            1.0,
        )
        .unwrap();
        assert_eq!(symbol.image.width(), 62);
        assert_eq!(symbol.image.height(), 99);
        assert_eq!(symbol.image.bytes().len(), 62 * 99 * 4);
    }
}
