use galileo_types::cartesian::Point3;
use galileo_types::geometry::Geom;

use crate::render::render_bundle::RenderBundle;
use crate::symbol::{CirclePointSymbol, SimpleContourSymbol, SimplePolygonSymbol, Symbol};
use crate::view::MapView;
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
    fn render(
        &self,
        feature: &F,
        geometry: &Geom<Point3>,
        min_resolution: f64,
        bundle: &mut RenderBundle,
        view: &MapView,
    ) {
        match geometry {
            Geom::Point(_) => self
                .point
                .render(feature, geometry, min_resolution, bundle, view),
            Geom::MultiPoint(_) => self
                .point
                .render(feature, geometry, min_resolution, bundle, view),
            Geom::Contour(_) => self
                .contour
                .render(feature, geometry, min_resolution, bundle, view),
            Geom::MultiContour(_) => self
                .contour
                .render(feature, geometry, min_resolution, bundle, view),
            Geom::Polygon(_) => self
                .polygon
                .render(feature, geometry, min_resolution, bundle, view),
            Geom::MultiPolygon(_) => self
                .polygon
                .render(feature, geometry, min_resolution, bundle, view),
        }
    }
}
