use crate::geo::traits::point::{GeoPoint, NewGeoPoint};
use crate::geo::traits::projection::Projection;
use crate::geometry::{Geom, Geometry};
use crate::point::{GeoPointType, Point};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GeoPoint2d {
    lat: f64,
    lon: f64,
}

impl GeoPoint for GeoPoint2d {
    type Num = f64;

    fn lat(&self) -> f64 {
        self.lat
    }

    fn lon(&self) -> f64 {
        self.lon
    }
}

impl NewGeoPoint<f64> for GeoPoint2d {
    fn latlon(lat: f64, lon: f64) -> Self {
        Self { lat, lon }
    }
}

impl GeoPoint2d {
    pub fn from(other: &impl GeoPoint<Num = f64>) -> Self {
        Self {
            lat: other.lat(),
            lon: other.lon(),
        }
    }
}

impl Point for GeoPoint2d {
    type Type = GeoPointType;
    type Num = f64;
    const DIMENSIONS: usize = 2;
}

impl Geometry for GeoPoint2d {
    type Point = Self;

    fn project<P: Projection<InPoint = Self::Point> + ?Sized>(
        &self,
        projection: &P,
    ) -> Option<Geom<P::OutPoint>> {
        Some(Geom::Point(projection.project(self)?))
    }
}

#[macro_export]
macro_rules! latlon {
    ($lat:expr, $lon:expr) => {
        <galileo_types::geo::impls::point::GeoPoint2d as galileo_types::geo::traits::point::NewGeoPoint<f64>>::latlon($lat, $lon)
    };
}
