#[cfg(not(target_arch = "wasm32"))]
use std::ops::Deref;
use std::sync::Arc;

use galileo_types::cartesian::Point3d;
use galileo_types::geometry::Geom;
use galileo_types::MultiPoint;
use image::EncodableLayout;
use nalgebra::Vector2;

use crate::decoded_image::DecodedImage;
use crate::error::GalileoError;
use crate::layer::feature_layer::symbol::Symbol;
use crate::render::point_paint::PointPaint;
use crate::render::render_bundle::RenderBundle;
use crate::Color;

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
    fn render(
        &self,
        _feature: &F,
        geometry: &Geom<Point3d>,
        min_resolution: f64,
        bundle: &mut RenderBundle,
    ) {
        let paint = PointPaint::circle(self.color, self.size as f32);
        match geometry {
            Geom::Point(point) => {
                bundle.add_point(point, &paint, min_resolution);
            }
            Geom::MultiPoint(points) => {
                points.iter_points().for_each(|p| {
                    bundle.add_point(p, &paint, min_resolution);
                });
            }
            _ => {}
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
        use galileo_types::cartesian::Size;

        let image = image::io::Reader::open(path)?
            .decode()
            .map_err(|_| GalileoError::ImageDecode)?;

        Ok(Self {
            image: Arc::new(DecodedImage::from_raw(
                Vec::from(image.to_rgba8().deref()),
                Size::new(image.width(), image.height()),
            )?),
            offset,
            scale,
        })
    }

    /// Decodes the image from the raw bytes.
    pub fn from_bytes(data: &[u8], offset: Vector2<f32>, scale: f32) -> Result<Self, GalileoError> {
        use galileo_types::cartesian::Size;

        let image = image::load_from_memory(data)
            .map_err(|_| GalileoError::ImageDecode)?
            .to_rgba8();

        Ok(Self {
            image: Arc::new(DecodedImage::from_raw(
                Vec::from(image.as_bytes()),
                Size::new(image.width(), image.height()),
            )?),
            offset,
            scale,
        })
    }
}

impl<F> Symbol<F> for ImagePointSymbol {
    fn render(
        &self,
        _feature: &F,
        geometry: &Geom<Point3d>,
        min_resolution: f64,
        bundle: &mut RenderBundle,
    ) {
        let paint = PointPaint::image(self.image.clone(), self.offset, self.scale);

        match geometry {
            Geom::Point(point) => {
                bundle.add_point(point, &paint, min_resolution);
            }
            Geom::MultiPoint(points) => points.iter_points().for_each(|point| {
                bundle.add_point(point, &paint, min_resolution);
            }),
            _ => {}
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
        assert_eq!(symbol.image.size(), 62 * 99 * 4);
    }
}
