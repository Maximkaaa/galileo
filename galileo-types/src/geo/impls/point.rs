use serde::{Deserialize, Serialize};

use crate::geo::traits::point::{GeoPoint, NewGeoPoint};
use crate::geometry_type::{GeoSpace2d, GeometryType, PointGeometryType};

/// 2d point on the surface of a celestial body.
#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct GeoPoint2d {
    lat: f64,
    lon: f64,
}

impl GeoPoint for GeoPoint2d {
    /// Numeric type used to represent coordinates.
    type Num = f64;

    /// Latitude in degrees.
    fn lat(&self) -> f64 {
        self.lat
    }

    /// Longitude in degrees.
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
    /// Creates a new from point from another.
    pub fn from(other: &impl GeoPoint<Num = f64>) -> Self {
        Self {
            lat: other.lat(),
            lon: other.lon(),
        }
    }
}

impl GeometryType for GeoPoint2d {
    type Type = PointGeometryType;
    type Space = GeoSpace2d;
}

/// Creates a new GeoPoint2d from latitude and longitude values (in degrees).
///
/// ```
/// use galileo_types::geo::GeoPoint;
/// use galileo_types::latlon;
///
/// let point = latlon!(38.0, 52.0);
/// assert_eq!(point.lat(), 38.0);
/// ```
#[macro_export]
macro_rules! latlon {
    ($lat:expr, $lon:expr) => {
        <$crate::geo::impls::GeoPoint2d as $crate::geo::NewGeoPoint<f64>>::latlon($lat, $lon)
    };
}
