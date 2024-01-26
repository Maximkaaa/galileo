use crate::layer::feature_layer::symbol::Symbol;
use crate::render::render_bundle::RenderBundle;
use crate::render::{LineCap, LinePaint, PrimitiveId};
use crate::Color;
use galileo_types::cartesian::traits::cartesian_point::CartesianPoint3d;
use galileo_types::geometry::Geom;
use galileo_types::multi_contour::MultiContour;
use num_traits::AsPrimitive;

pub struct SimpleContourSymbol {
    pub color: Color,
    pub width: f64,
}

impl SimpleContourSymbol {
    pub fn new(color: Color, width: f64) -> Self {
        Self { color, width }
    }
}

impl<F> Symbol<F> for SimpleContourSymbol {
    fn render<N: AsPrimitive<f32>, P: CartesianPoint3d<Num = N>>(
        &self,
        _feature: &F,
        geometry: &Geom<P>,
        bundle: &mut RenderBundle,
    ) -> Vec<PrimitiveId> {
        let paint = LinePaint {
            color: self.color,
            width: self.width,
            offset: 0.0,
            line_cap: LineCap::Butt,
        };

        match geometry {
            Geom::Contour(contour) => vec![bundle.add_line(contour, paint)],
            Geom::MultiContour(contours) => contours
                .contours()
                .map(|contour| bundle.add_line(contour, paint))
                .collect(),
            _ => vec![],
        }
    }
}
