use crate::render::render_bundle::RenderPrimitive;
use crate::symbol::{CirclePointSymbol, SimpleContourSymbol, SimplePolygonSymbol, Symbol};
use crate::Color;
use galileo_types::cartesian::impls::contour::Contour;
use galileo_types::cartesian::impls::polygon::Polygon;
use galileo_types::cartesian::traits::cartesian_point::CartesianPoint3d;
use galileo_types::geometry::Geom;
use num_traits::AsPrimitive;

#[derive(Debug, Clone)]
pub struct ArbitraryGeometrySymbol {
    point: CirclePointSymbol,
    contour: SimpleContourSymbol,
    polygon: SimplePolygonSymbol,
}

impl ArbitraryGeometrySymbol {
    pub fn new(
        point: CirclePointSymbol,
        contour: SimpleContourSymbol,
        polygon: SimplePolygonSymbol,
    ) -> Self {
        Self {
            point,
            contour,
            polygon,
        }
    }
}

impl Default for ArbitraryGeometrySymbol {
    fn default() -> Self {
        Self {
            point: CirclePointSymbol::new(Color::RED, 5.0),
            contour: SimpleContourSymbol::new(Color::GREEN, 2.0),
            polygon: SimplePolygonSymbol::new(Color::BLUE),
        }
    }
}

impl<F> Symbol<F> for ArbitraryGeometrySymbol {
    fn render<'a, N, P>(
        &self,
        feature: &F,
        geometry: &'a Geom<P>,
        min_resolution: f64,
    ) -> Vec<RenderPrimitive<'a, N, P, Contour<P>, Polygon<P>>>
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N> + Clone,
    {
        match geometry {
            Geom::Point(_) => self.point.render(feature, geometry, min_resolution),
            Geom::MultiPoint(_) => self.point.render(feature, geometry, min_resolution),
            Geom::Contour(_) => self.contour.render(feature, geometry, min_resolution),
            Geom::MultiContour(_) => self.contour.render(feature, geometry, min_resolution),
            Geom::Polygon(_) => self.polygon.render(feature, geometry, min_resolution),
            Geom::MultiPolygon(_) => self.polygon.render(feature, geometry, min_resolution),
        }
    }
}
