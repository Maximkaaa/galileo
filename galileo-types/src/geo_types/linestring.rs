use geo_types::{Coord, CoordNum, LineString};

use crate::contour::Contour;
use crate::geometry_type::{AmbiguousSpace, ContourGeometryType, GeometryType};

impl<T: CoordNum> Contour for LineString<T> {
    type Point = Coord<T>;

    fn is_closed(&self) -> bool {
        LineString::is_closed(self)
    }

    fn iter_points(&self) -> impl Iterator<Item = &'_ Self::Point> {
        if self.is_closed() {
            self.0[..(self.0.len().max(1) - 1)].iter()
        } else {
            self.0.iter()
        }
    }
}

impl<T: CoordNum> GeometryType for LineString<T> {
    type Type = ContourGeometryType;
    type Space = AmbiguousSpace;
}
