use crate::layer::feature_layer::symbol::Symbol;
use crate::render::render_bundle::RenderPrimitive;
use crate::render::{LineCap, LinePaint};
use crate::Color;
use galileo_types::cartesian::CartesianPoint3d;
use galileo_types::geometry::Geom;
use galileo_types::impls::{Contour, Polygon};
use galileo_types::MultiContour;
use num_traits::AsPrimitive;

/// Renders a contour as a line of fixed width.
#[derive(Debug, Copy, Clone)]
pub struct SimpleContourSymbol {
    /// Color of the line.
    pub color: Color,
    /// Width of the line in pixels.
    pub width: f64,
}

impl SimpleContourSymbol {
    /// Creates a new instance.
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
