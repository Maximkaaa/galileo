use crate::error::GalileoError;
use crate::layer::feature_layer::symbol::Symbol;
use crate::primitives::DecodedImage;
use crate::render::point_paint::PointPaint;
use crate::render::render_bundle::RenderBundle;
use crate::render::PrimitiveId;
use crate::Color;
use galileo_types::cartesian::traits::cartesian_point::CartesianPoint3d;
use galileo_types::geometry::Geom;
use galileo_types::multi_point::MultiPoint;
use nalgebra::Vector2;
use num_traits::AsPrimitive;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug, Copy, Clone)]
pub struct CirclePointSymbol {
    pub color: Color,
    pub size: f64,
}

impl CirclePointSymbol {
    pub fn new(color: Color, size: f64) -> Self {
        Self { color, size }
    }
}

impl<F> Symbol<F> for CirclePointSymbol {
    fn render<N: AsPrimitive<f32>, P: CartesianPoint3d<Num = N>>(
        &self,
        _feature: &F,
        geometry: &Geom<P>,
        bundle: &mut RenderBundle,
        _min_resolution: f64,
    ) -> Vec<PrimitiveId> {
        let paint = PointPaint::circle(self.color, self.size as f32);

        match geometry {
            Geom::Point(point) => vec![bundle.add_point(point, paint)],
            Geom::MultiPoint(points) => points
                .iter_points()
                .map(|point| bundle.add_point(point, paint.clone()))
                .collect(),
            _ => vec![],
        }
    }
}

pub struct ImagePointSymbol {
    image: Arc<DecodedImage>,
    offset: Vector2<f32>,
    scale: f32,
}

impl ImagePointSymbol {
    #[cfg(feature = "image")]
    pub fn from_path(path: &str, offset: Vector2<f32>, scale: f32) -> Result<Self, GalileoError> {
        use image::GenericImageView;
        let image = image::io::Reader::open(path)?.decode()?;
        let dimensions = image.dimensions();

        Ok(Self {
            image: Arc::new(DecodedImage {
                bytes: Vec::from(image.to_rgba8().deref()),
                dimensions,
            }),
            offset,
            scale,
        })
    }
}

impl<F> Symbol<F> for ImagePointSymbol {
    fn render<N: AsPrimitive<f32>, P: CartesianPoint3d<Num = N>>(
        &self,
        _feature: &F,
        geometry: &Geom<P>,
        bundle: &mut RenderBundle,
        _min_resolution: f64,
    ) -> Vec<PrimitiveId> {
        let paint = PointPaint::image(self.image.clone(), self.offset, self.scale);

        match geometry {
            Geom::Point(point) => vec![bundle.add_point(point, paint)],
            Geom::MultiPoint(points) => points
                .iter_points()
                .map(|point| bundle.add_point(point, paint.clone()))
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
        assert_eq!(symbol.image.dimensions, (62, 99));
        assert_eq!(symbol.image.bytes.len(), 62 * 99 * 4);
    }
}
