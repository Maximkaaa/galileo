use crate::contour::Contour;
use crate::geo::traits::projection::Projection;
use crate::geometry::{Geom, Geometry, GeometrySpecialization};
use crate::geometry_type::{GeometryType, PolygonGeometryType};
use crate::segment::Segment;

pub trait Polygon {
    type Contour: Contour;

    fn outer_contour(&self) -> &Self::Contour;
    fn inner_contours(&self) -> impl Iterator<Item = &'_ Self::Contour>;

    fn iter_contours(&self) -> impl Iterator<Item = &'_ Self::Contour> {
        Box::new(std::iter::once(self.outer_contour()).chain(self.inner_contours()))
    }

    fn iter_segments(
        &self,
    ) -> impl Iterator<Item = Segment<'_, <Self::Contour as Contour>::Point>> {
        Box::new(self.iter_contours().flat_map(Self::Contour::iter_segments))
    }
}

impl<Poly, Space> GeometrySpecialization<PolygonGeometryType, Space> for Poly
where
    Poly: Polygon + GeometryType<Type = PolygonGeometryType, Space = Space>,
    Poly::Contour: Geometry,
{
    type Point = <Poly::Contour as Geometry>::Point;

    fn project<Proj>(&self, projection: &Proj) -> Option<Geom<Proj::OutPoint>>
    where
        Proj: Projection<InPoint = Self::Point> + ?Sized,
    {
        let Geom::Contour(outer_contour) = self.outer_contour().project(projection)? else {
            return None;
        };
        let inner_contours = self
            .inner_contours()
            .map(|c| {
                c.project(projection).and_then(|c| match c {
                    Geom::Contour(contour) => contour.into_closed(),
                    _ => None,
                })
            })
            .collect::<Option<Vec<crate::cartesian::impls::contour::ClosedContour<Proj::OutPoint>>>>()?;
        Some(Geom::Polygon(crate::cartesian::impls::polygon::Polygon {
            outer_contour: outer_contour.into_closed()?,
            inner_contours,
        }))
    }
}
