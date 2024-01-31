use crate::cartesian::impls::polygon::Polygon;
use crate::geometry_type::{GeometryType, MultiPolygonGeometryType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MultiPolygon<P> {
    pub parts: Vec<Polygon<P>>,
}

impl<P> From<Vec<Polygon<P>>> for MultiPolygon<P> {
    fn from(parts: Vec<Polygon<P>>) -> Self {
        Self { parts }
    }
}

impl<P> MultiPolygon<P> {
    pub fn parts(&self) -> &[Polygon<P>] {
        &self.parts
    }
}

impl<P> crate::multi_polygon::MultiPolygon for MultiPolygon<P> {
    type Polygon = Polygon<P>;

    fn polygons(&self) -> impl Iterator<Item = &Self::Polygon> {
        self.parts.iter()
    }
}

impl<P: GeometryType> GeometryType for MultiPolygon<P> {
    type Type = MultiPolygonGeometryType;
    type Space = P::Space;
}
