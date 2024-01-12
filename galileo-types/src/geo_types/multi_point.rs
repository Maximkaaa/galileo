use crate::geometry_type::{AmbiguousSpace, GeometryType, MultiPointGeometryType};
use geo_types::{CoordNum, MultiPoint, Point};

impl<T: CoordNum> crate::multi_point::MultiPoint for MultiPoint<T> {
    type Point = Point<T>;

    fn iter_points(&self) -> impl Iterator<Item = &'_ Self::Point> {
        self.0.iter()
    }
}

impl<T: CoordNum> GeometryType for MultiPoint<T> {
    type Type = MultiPointGeometryType;
    type Space = AmbiguousSpace;
}
