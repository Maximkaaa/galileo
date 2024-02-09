use crate::layer::feature_layer::symbol::Symbol;
use crate::render::render_bundle::RenderPrimitive;
use crate::render::{LineCap, LinePaint};
use crate::Color;
use galileo_types::cartesian::impls::contour::Contour;
use galileo_types::cartesian::impls::polygon::Polygon;
use galileo_types::cartesian::traits::cartesian_point::CartesianPoint3d;
use galileo_types::geometry::Geom;
use galileo_types::multi_contour::MultiContour;
use num_traits::AsPrimitive;

#[derive(Debug, Copy, Clone)]
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
        let paint = LinePaint {
            color: self.color,
            width: self.width,
            offset: 0.0,
            line_cap: LineCap::Butt,
        };

        match geometry {
            Geom::Contour(contour) => vec![RenderPrimitive::new_contour_ref(contour, paint)],
            Geom::MultiContour(contours) => contours
                .contours()
                .map(|contour| RenderPrimitive::new_contour_ref(contour, paint))
                .collect(),
            _ => vec![],
        }
    }
}
