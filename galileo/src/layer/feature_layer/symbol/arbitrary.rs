use galileo_types::cartesian::CartesianPoint3d;
use galileo_types::geometry::Geom;
use galileo_types::impls::{Contour, Polygon};
use num_traits::AsPrimitive;

use crate::render::render_bundle::RenderPrimitive;
use crate::symbol::{CirclePointSymbol, SimpleContourSymbol, SimplePolygonSymbol, Symbol};
use crate::Color;

/// Renders any type of the geometry with the set inner symbols.
#[derive(Debug, Clone)]
pub struct ArbitraryGeometrySymbol {
    point: CirclePointSymbol,
    contour: SimpleContourSymbol,
    polygon: SimplePolygonSymbol,
}

impl ArbitraryGeometrySymbol {
    /// Creates a new symbol. Geometries of corresponding types will use one of the given symbol to be drawn.
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
