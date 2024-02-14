use crate::cartesian::{CartesianPoint2d, NewCartesianPoint2d};
use crate::geo::{GeoPoint, NewGeoPoint};
use crate::geometry_type::{AmbiguousSpace, GeometryType, PointGeometryType};
use geo_types::{point, CoordNum};
use nalgebra::Scalar;
use num_traits::{Bounded, Float, FromPrimitive};

impl<T: CoordNum + Bounded + Scalar + FromPrimitive> CartesianPoint2d for geo_types::Point<T> {
    type Num = T;

    fn x(&self) -> Self::Num {
        self.0.x
    }

    fn y(&self) -> Self::Num {
        self.0.y
    }
}

impl<T: CoordNum + Bounded + Scalar + FromPrimitive> NewCartesianPoint2d<T>
    for geo_types::Point<T>
{
    fn new(x: T, y: T) -> Self {
        point!(x: x, y: y)
    }
}

impl<T: CoordNum + Bounded + Scalar + FromPrimitive> GeometryType for geo_types::Point<T> {
    type Type = PointGeometryType;
    type Space = AmbiguousSpace;
}

impl<T: CoordNum + Float> GeoPoint for geo_types::Point<T> {
    type Num = T;

    fn lat(&self) -> Self::Num {
        self.y()
    }

    fn lon(&self) -> Self::Num {
        self.x()
    }
}

impl<T: CoordNum + Float> NewGeoPoint<T> for geo_types::Point<T> {
    fn latlon(lat: T, lon: T) -> Self {
        point!(x: lon, y: lat)
    }
}
