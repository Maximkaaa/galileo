use galileo::layer::feature::feature::Feature;
use galileo::primitives::Color;
use galileo_types::cartesian::impls::multipolygon::MultiPolygon;
use galileo_types::cartesian::impls::point::Point2d;
use galileo_types::cartesian::impls::polygon::Polygon;
use galileo_types::cartesian::rect::Rect;
use galileo_types::cartesian::traits::cartesian_point::CartesianPoint2d;
use galileo_types::geo::traits::projection::Projection;
use galileo_types::geometry::{CartesianGeometry2d, Geom, Geometry};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Country {
    pub name: String,
    #[serde(deserialize_with = "des_geometry")]
    pub geometry: MultiPolygon<Point2d>,
    pub color: Color,
    pub bbox: Rect,
    pub is_selected: bool,
}

fn des_geometry<'de, D: Deserializer<'de>>(d: D) -> Result<MultiPolygon<Point2d>, D::Error> {
    Ok(Vec::<Polygon<Point2d>>::deserialize(d)?.into())
}

impl Country {
    pub fn is_selected(&self) -> bool {
        self.is_selected
    }
}

impl Geometry for Country {
    type Point = Point2d;

    fn project<P: Projection<InPoint = Self::Point> + ?Sized>(
        &self,
        projection: &P,
    ) -> Option<Geom<P::OutPoint>> {
        self.geometry.project(projection)
    }
}

impl CartesianGeometry2d<Point2d> for Country {
    fn is_point_inside<Other: CartesianPoint2d<Num = f64>>(
        &self,
        point: &Other,
        tolerance: f64,
    ) -> bool {
        if !self.bbox.contains(point) {
            return false;
        }

        self.geometry.is_point_inside(point, tolerance)
    }

    fn bounding_rectangle(&self) -> Rect {
        self.bbox
    }
}

pub fn load_countries() -> Vec<Country> {
    bincode::deserialize(include_bytes!("countries.data")).unwrap()
}
