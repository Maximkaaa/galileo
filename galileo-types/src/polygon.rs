use crate::geometry::GeometryMarker;
use crate::traits::polygon::PolygonMarker;
use crate::ClosedContour;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Polygon<P> {
    pub outer_contour: ClosedContour<P>,
    pub inner_contours: Vec<ClosedContour<P>>,
}

impl<P> Polygon<P> {
    pub fn iter_contours(&self) -> impl Iterator<Item = &ClosedContour<P>> {
        std::iter::once(&self.outer_contour).chain(self.inner_contours.iter())
    }

    pub fn cast_points<T>(&self, cast: impl Fn(&P) -> T) -> Polygon<T> {
        Polygon {
            outer_contour: ClosedContour::new(
                self.outer_contour.points.iter().map(|p| cast(p)).collect(),
            ),
            inner_contours: self
                .inner_contours
                .iter()
                .map(|c| ClosedContour::new(c.points.iter().map(|p| cast(p)).collect()))
                .collect(),
        }
    }
}

impl<P> GeometryMarker for Polygon<P> {
    type Marker = PolygonMarker;
}

impl<P> crate::traits::polygon::Polygon for Polygon<P> {
    type Contour = ClosedContour<P>;

    fn outer_contour(&self) -> &Self::Contour {
        &self.outer_contour
    }

    fn inner_contours(&self) -> Box<dyn Iterator<Item = &'_ Self::Contour> + '_> {
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
