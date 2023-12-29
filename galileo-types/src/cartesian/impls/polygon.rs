use crate::cartesian::impls::contour::ClosedContour;
use crate::cartesian::rect::Rect;
use crate::cartesian::traits::cartesian_point::CartesianPoint2d;
use crate::cartesian::traits::polygon::CartesianPolygon;
use crate::geo::traits::projection::Projection;
use crate::geometry::{CartesianGeometry2d, Geom, Geometry};
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

impl<P> crate::cartesian::traits::polygon::Polygon for Polygon<P> {
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

impl<P> Geometry for Polygon<P> {
    type Point = P;

    fn project<Proj: Projection<InPoint = Self::Point> + ?Sized>(
        &self,
        projection: &Proj,
    ) -> Option<Geom<Proj::OutPoint>> {
        let Geom::Line(outer_contour) = self.outer_contour.project(projection)? else {
            return None;
        };
        let inner_contours = self
            .inner_contours
            .iter()
            .map(|c| {
                c.project(projection).and_then(|c| match c {
                    Geom::Line(contour) => contour.into_closed(),
                    _ => None,
                })
            })
            .collect::<Option<Vec<ClosedContour<Proj::OutPoint>>>>()?;
        Some(Geom::Polygon(Polygon {
            outer_contour: outer_contour.into_closed()?,
            inner_contours,
        }))
    }
}

impl<P> CartesianGeometry2d<P> for Polygon<P>
where
    P: CartesianPoint2d,
{
    fn is_point_inside<Other: CartesianPoint2d<Num = P::Num>>(
        &self,
        point: &Other,
        _tolerance: P::Num,
    ) -> bool {
        self.contains_point(point)
    }

    fn bounding_rectangle(&self) -> Rect<P::Num> {
        todo!()
    }
}
