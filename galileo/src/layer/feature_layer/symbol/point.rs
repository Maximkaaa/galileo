use crate::layer::feature_layer::symbol::Symbol;
use crate::render::point_paint::PointPaint;
use crate::render::render_bundle::RenderBundle;
use crate::render::PrimitiveId;
use crate::Color;
use galileo_types::cartesian::traits::cartesian_point::CartesianPoint3d;
use galileo_types::geometry::Geom;
use galileo_types::multi_point::MultiPoint;
use num_traits::AsPrimitive;

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
