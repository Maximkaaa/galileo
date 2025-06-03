use serde::{Deserialize, Serialize};

use crate::geometry_type::{GeometryType, MultiPolygonGeometryType};
use crate::impls::polygon::Polygon;

/// A set of polygons.
#[derive(Debug, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Hash, Deserialize, Serialize)]
pub struct MultiPolygon<P> {
    /// Inner polygons.
    pub parts: Vec<Polygon<P>>,
}

impl<P> From<Vec<Polygon<P>>> for MultiPolygon<P> {
    fn from(parts: Vec<Polygon<P>>) -> Self {
        Self { parts }
    }
}

impl<P> MultiPolygon<P> {
    /// Returns reference to the inner polygons.
    pub fn parts(&self) -> &[Polygon<P>] {
        &self.parts
    }
}

impl<P: Copy> crate::multi_polygon::MultiPolygon for MultiPolygon<P> {
    type Polygon = Polygon<P>;

    fn polygons(&self) -> impl Iterator<Item = &Self::Polygon> {
        self.parts.iter()
    }
}

impl<P: GeometryType> GeometryType for MultiPolygon<P> {
    type Type = MultiPolygonGeometryType;
    type Space = P::Space;
}
