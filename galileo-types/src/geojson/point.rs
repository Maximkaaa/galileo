use crate::error::GalileoTypesError;
use crate::geo::traits::point::{GeoPoint, NewGeoPoint};
use crate::geometry_type::{GeoSpace2d, GeometryType, PointGeometryType};
use geojson::Position;

pub struct GeoJsonPoint(Position);

impl TryFrom<Position> for GeoJsonPoint {
    type Error = GalileoTypesError;

    fn try_from(value: Position) -> Result<Self, Self::Error> {
        if value.len() < 2 {
            Err(GalileoTypesError::Conversion(
                "point must contain at least 2 dimensions".to_string(),
            ))
        } else {
            Ok(GeoJsonPoint(value))
        }
    }
}

impl GeometryType for GeoJsonPoint {
    type Type = PointGeometryType;
    type Space = GeoSpace2d;
}

impl GeoPoint for GeoJsonPoint {
    type Num = f64;

    fn lat(&self) -> Self::Num {
        self.0[1]
    }

    fn lon(&self) -> Self::Num {
        self.0[0]
    }
}

impl NewGeoPoint for GeoJsonPoint {
    fn latlon(lat: f64, lon: f64) -> Self {
        Self(vec![lon, lat])
    }
}
