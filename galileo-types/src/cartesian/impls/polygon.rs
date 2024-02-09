use crate::cartesian::impls::contour::ClosedContour;
use crate::geometry_type::{GeometryType, PolygonGeometryType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Polygon<P> {
    pub outer_contour: ClosedContour<P>,
    pub inner_contours: Vec<ClosedContour<P>>,
}

impl<P> Polygon<P> {
    pub fn new(outer_contour: ClosedContour<P>, inner_contours: Vec<ClosedContour<P>>) -> Self {
        Self {
            outer_contour,
            inner_contours,
        }
    }

    pub fn iter_contours(&self) -> impl Iterator<Item = &ClosedContour<P>> {
        std::iter::once(&self.outer_contour).chain(self.inner_contours.iter())
    }

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
