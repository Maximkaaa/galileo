use galileo::layer::feature_layer::Feature;
use galileo::Color;
use galileo_types::cartesian::{CartesianPoint2d, Point2d, Rect};
use galileo_types::geo::impls::GeoPoint2d;
use galileo_types::geo::{GeoPoint, NewGeoPoint, Projection};
use galileo_types::geometry::{CartesianGeometry2d, Geom, Geometry};
use galileo_types::impls::{MultiPolygon, Polygon};
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
    #[allow(dead_code)]
    pub capital: String,
    #[allow(dead_code)]
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
