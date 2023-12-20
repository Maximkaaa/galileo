use crate::geo::traits::point::{GeoPoint, NewGeoPoint};
use crate::geometry::{GeoPointType, Point};

#[derive(Debug, Clone, Copy, Default)]
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
