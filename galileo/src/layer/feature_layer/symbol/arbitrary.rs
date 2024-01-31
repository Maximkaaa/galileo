use crate::render::render_bundle::RenderBundle;
use crate::render::PrimitiveId;
use crate::symbol::{CirclePointSymbol, SimpleContourSymbol, SimplePolygonSymbol, Symbol};
use crate::Color;
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
    fn render<N: AsPrimitive<f32>, P: CartesianPoint3d<Num = N>>(
        &self,
        feature: &F,
        geometry: &Geom<P>,
        bundle: &mut RenderBundle,
        min_resolution: f64,
    ) -> Vec<PrimitiveId> {
        match geometry {
            Geom::Point(_) => self.point.render(feature, geometry, bundle, min_resolution),
            Geom::MultiPoint(_) => self.point.render(feature, geometry, bundle, min_resolution),
            Geom::Contour(_) => self
                .contour
                .render(feature, geometry, bundle, min_resolution),
            Geom::MultiContour(_) => self
                .contour
                .render(feature, geometry, bundle, min_resolution),
            Geom::Polygon(_) => self
                .polygon
                .render(feature, geometry, bundle, min_resolution),
            Geom::MultiPolygon(_) => self
                .polygon
                .render(feature, geometry, bundle, min_resolution),
        }
    }
}
