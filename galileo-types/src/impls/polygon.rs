use serde::{Deserialize, Serialize};

use crate::geometry_type::{GeometryType, PolygonGeometryType};
use crate::impls::contour::ClosedContour;

/// Simple implementation of the [`Polygon`](crate::Polygon) trait.
#[derive(Debug, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Hash, Deserialize, Serialize)]
pub struct Polygon<P> {
    /// Outer contour.
    pub outer_contour: ClosedContour<P>,
    /// Inner contours.
    pub inner_contours: Vec<ClosedContour<P>>,
}

impl<P> Polygon<P> {
    /// Creates a new polygon.
    pub fn new(outer_contour: ClosedContour<P>, inner_contours: Vec<ClosedContour<P>>) -> Self {
        Self {
            outer_contour,
            inner_contours,
        }
    }

    /// Casts all points of the polygon into a different numeric type.
    pub fn cast_points<T>(&self, mut cast: impl Fn(&P) -> T) -> Polygon<T> {
        Polygon {
            outer_contour: ClosedContour::new(
                self.outer_contour.points.iter().map(&mut cast).collect(),
            ),
            inner_contours: self
                .inner_contours
                .iter()
                .map(|c| ClosedContour::new(c.points.iter().map(&mut cast).collect()))
                .collect(),
        }
    }
}

impl<P> crate::polygon::Polygon for Polygon<P> {
    type Contour = ClosedContour<P>;

    fn outer_contour(&self) -> &Self::Contour {
        &self.outer_contour
    }

    fn inner_contours(&self) -> impl Iterator<Item = &'_ Self::Contour> {
        Box::new(self.inner_contours.iter())
    }
}

impl<P> From<ClosedContour<P>> for Polygon<P> {
    fn from(value: ClosedContour<P>) -> Self {
        Self {
            outer_contour: value,
            inner_contours: vec![],
        }
    }
}

impl<P: GeometryType> GeometryType for Polygon<P> {
    type Type = PolygonGeometryType;
    type Space = P::Space;
}

impl<P> From<Vec<P>> for Polygon<P> {
    fn from(value: Vec<P>) -> Self {
        Self {
            outer_contour: ClosedContour::new(value),
            inner_contours: vec![],
        }
    }
}
