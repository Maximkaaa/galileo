use serde::{Deserialize, Serialize};

use crate::geometry_type::{GeometryType, MultiPointGeometryType};

/// A set of points.
#[derive(Debug, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Hash, Deserialize, Serialize)]
pub struct MultiPoint<P>(Vec<P>);

impl<P: Copy> crate::multi_point::MultiPoint for MultiPoint<P> {
    type Point = P;

    fn iter_points(&self) -> impl Iterator<Item = Self::Point> {
        self.0.iter().copied()
    }
}

impl<P> From<Vec<P>> for MultiPoint<P> {
    fn from(value: Vec<P>) -> Self {
        Self(value)
    }
}

impl<P: GeometryType> GeometryType for MultiPoint<P> {
    type Type = MultiPointGeometryType;
    type Space = P::Space;
}
