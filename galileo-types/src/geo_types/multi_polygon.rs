use geo_types::{CoordNum, MultiPolygon, Polygon};

use crate::geometry_type::{AmbiguousSpace, GeometryType, MultiPolygonGeometryType};

impl<T: CoordNum> crate::multi_polygon::MultiPolygon for MultiPolygon<T> {
    type Polygon = Polygon<T>;

    fn polygons(&self) -> impl Iterator<Item = &Self::Polygon> {
        self.0.iter()
    }
}

impl<T: CoordNum> GeometryType for MultiPolygon<T> {
    type Type = MultiPolygonGeometryType;
    type Space = AmbiguousSpace;
}
