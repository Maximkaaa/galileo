use geo_types::{CoordNum, LineString};

use crate::geometry_type::{AmbiguousSpace, GeometryType, PolygonGeometryType};
use crate::polygon::Polygon;

impl<T: CoordNum> Polygon for geo_types::Polygon<T> {
    type Contour = LineString<T>;

    fn outer_contour(&self) -> &Self::Contour {
        self.exterior()
    }

    fn inner_contours(&self) -> impl Iterator<Item = &'_ Self::Contour> {
        self.interiors().iter()
    }
}

impl<T: CoordNum> GeometryType for geo_types::Polygon<T> {
    type Type = PolygonGeometryType;
    type Space = AmbiguousSpace;
}
