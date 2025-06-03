use serde::{Deserialize, Serialize};

use crate::geometry_type::{GeometryType, MultiContourGeometryType};
use crate::impls::contour::Contour;

/// A set of contours.
#[derive(Debug, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Hash, Deserialize, Serialize)]
pub struct MultiContour<P>(Vec<Contour<P>>);

impl<P: Copy> crate::multi_contour::MultiContour for MultiContour<P> {
    type Contour = Contour<P>;

    fn contours(&self) -> impl Iterator<Item = &Self::Contour> {
        self.0.iter()
    }
}

impl<P> From<Vec<Contour<P>>> for MultiContour<P> {
    fn from(value: Vec<Contour<P>>) -> Self {
        Self(value)
    }
}

impl<P: GeometryType> GeometryType for MultiContour<P> {
    type Type = MultiContourGeometryType;
    type Space = P::Space;
}
