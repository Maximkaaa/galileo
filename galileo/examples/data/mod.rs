use galileo::layer::feature_layer::feature::Feature;
use galileo::Color;
use galileo_types::cartesian::impls::point::Point2d;
use galileo_types::cartesian::traits::cartesian_point::CartesianPoint2d;
use galileo_types::cartesian::Rect;
use galileo_types::geo::impls::point::GeoPoint2d;
use galileo_types::geo::traits::point::{GeoPoint, NewGeoPoint};
use galileo_types::geo::traits::projection::Projection;
use galileo_types::geometry::{CartesianGeometry2d, Geom, Geometry};
use galileo_types::impls::multi_polygon::MultiPolygon;
use galileo_types::impls::polygon::Polygon;
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

impl Feature for Country {
    type Geom = Self;

    fn geometry(&self) -> &Self::Geom {
        self
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

    fn bounding_rectangle(&self) -> Option<Rect> {
        Some(self.bbox)
    }
}

#[derive(Debug, Deserialize)]
pub struct City {
    lat: f64,
    lng: f64,
    pub capital: String,
    pub population: f64,
}

impl Feature for City {
    type Geom = Self;

    fn geometry(&self) -> &Self::Geom {
        self
    }
}

impl GeoPoint for City {
    type Num = f64;

    fn lat(&self) -> Self::Num {
        self.lat
    }

    fn lon(&self) -> Self::Num {
        self.lng
    }
}

impl Geometry for City {
    type Point = GeoPoint2d;

    fn project<P: Projection<InPoint = Self::Point> + ?Sized>(
        &self,
        projection: &P,
    ) -> Option<Geom<P::OutPoint>> {
        GeoPoint2d::latlon(self.lat, self.lng).project(projection)
    }
}
