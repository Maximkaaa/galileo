use geo_types::{coord, Coord, CoordNum};
use nalgebra::Scalar;
use num_traits::{Bounded, Float, FromPrimitive};

use crate::cartesian::{CartesianPoint2d, NewCartesianPoint2d};
use crate::geo::{GeoPoint, NewGeoPoint};
use crate::geometry_type::{AmbiguousSpace, GeometryType, PointGeometryType};

impl<T: CoordNum + Bounded + Scalar + FromPrimitive> CartesianPoint2d for Coord<T> {
    type Num = T;

    fn x(&self) -> Self::Num {
        self.x
    }

    fn y(&self) -> Self::Num {
        self.y
    }
}

impl<T: CoordNum + Bounded + Scalar + FromPrimitive> NewCartesianPoint2d<T> for Coord<T> {
    fn new(x: T, y: T) -> Self {
        coord!(x: x, y: y)
    }
}

impl<T: CoordNum + Bounded + Scalar + FromPrimitive> GeometryType for Coord<T> {
    type Type = PointGeometryType;
    type Space = AmbiguousSpace;
}

impl<T: CoordNum + Float> GeoPoint for Coord<T> {
    type Num = T;

    fn lat(&self) -> Self::Num {
        self.y
    }

    fn lon(&self) -> Self::Num {
        self.x
    }
}

impl<T: CoordNum + Float> NewGeoPoint<T> for Coord<T> {
    fn latlon(lat: T, lon: T) -> Self {
        coord!(x: lon, y: lat)
    }
}
