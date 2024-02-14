use crate::cartesian::Rect;
use crate::cartesian::{CartesianPoint2d, CartesianPolygon};
use crate::contour::Contour;
use crate::geo::Projection;
use crate::geometry::{
    CartesianGeometry2d, CartesianGeometry2dSpecialization, Geom, Geometry, GeometrySpecialization,
};
use crate::geometry_type::{CartesianSpace2d, GeometryType, PolygonGeometryType};
use crate::segment::Segment;

/// Polygon geometry. Polygon consists of one outer contour, and zero or more inner contours.
///
/// Zero contours represent *holes* in a polygon. If one inner contour is inside another inner contour, it represents
/// non-hole area inside a hole.
pub trait Polygon {
    /// Contour type.
    type Contour: Contour;

    /// Outer contour of the polygon.
    fn outer_contour(&self) -> &Self::Contour;
    /// iterates over inner contours.
    fn inner_contours(&self) -> impl Iterator<Item = &'_ Self::Contour>;

    /// Iterates over all contours of the polygon starting with the outer one.
    fn iter_contours(&self) -> impl Iterator<Item = &'_ Self::Contour> {
        Box::new(std::iter::once(self.outer_contour()).chain(self.inner_contours()))
    }

    /// Iterates over all segments of the polygon contour lines.
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

    fn project_spec<Proj>(&self, projection: &Proj) -> Option<Geom<Proj::OutPoint>>
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
            .collect::<Option<Vec<crate::impls::ClosedContour<Proj::OutPoint>>>>()?;
        Some(Geom::Polygon(crate::impls::Polygon {
            outer_contour: outer_contour.into_closed()?,
            inner_contours,
        }))
    }
}

impl<P, Poly> CartesianGeometry2dSpecialization<P, PolygonGeometryType> for Poly
where
    P: CartesianPoint2d,
    Poly: Polygon
        + CartesianPolygon<Point = P>
        + GeometryType<Type = PolygonGeometryType, Space = CartesianSpace2d>
        + Geometry<Point = P>,
    Poly::Contour: Contour<Point = P> + CartesianGeometry2d<P>,
{
    fn is_point_inside_spec<Other: CartesianPoint2d<Num = P::Num>>(
        &self,
        point: &Other,
        _tolerance: P::Num,
    ) -> bool {
        self.contains_point(point)
    }

    fn bounding_rectangle_spec(&self) -> Option<Rect<P::Num>> {
        self.iter_contours()
            .filter_map(|c| c.bounding_rectangle())
            .collect()
    }
}
