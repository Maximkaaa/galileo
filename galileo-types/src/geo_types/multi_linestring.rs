use crate::geometry_type::{AmbiguousSpace, GeometryType, MultiContourGeometryType};
use crate::multi_contour::MultiContour;
use geo_types::{CoordNum, LineString, MultiLineString};

impl<T: CoordNum> MultiContour for MultiLineString<T> {
    type Contour = LineString<T>;

    fn contours(&self) -> impl Iterator<Item = &Self::Contour> {
        self.0.iter()
    }
}

impl<T: CoordNum> GeometryType for MultiLineString<T> {
    type Type = MultiContourGeometryType;
    type Space = AmbiguousSpace;
}
