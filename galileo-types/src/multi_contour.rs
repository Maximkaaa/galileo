use crate::cartesian::{CartesianPoint2d, Rect};
use crate::contour::Contour;
use crate::geo::Projection;
use crate::geometry::{
    CartesianGeometry2d, CartesianGeometry2dSpecialization, Geom, Geometry, GeometrySpecialization,
};
use crate::geometry_type::{CartesianSpace2d, GeometryType, MultiContourGeometryType};

/// Geometry consisting of several contours.
pub trait MultiContour {
    /// Contour type.
    type Contour: Contour;

    /// Iterator over contours.
    fn contours(&self) -> impl Iterator<Item = &Self::Contour>;
}

impl<C, Space> GeometrySpecialization<MultiContourGeometryType, Space> for C
where
    C: MultiContour + GeometryType<Type = MultiContourGeometryType, Space = Space>,
    C::Contour: Geometry,
{
    type Point = <C::Contour as Geometry>::Point;

    fn project_spec<Proj>(&self, projection: &Proj) -> Option<Geom<Proj::OutPoint>>
    where
        Proj: Projection<InPoint = Self::Point> + ?Sized,
    {
        let contours = self
            .contours()
            .map(|c| {
                c.project(projection).and_then(|c| match c {
                    Geom::Contour(contour) => Some(contour),
                    _ => None,
                })
            })
            .collect::<Option<Vec<crate::impls::Contour<Proj::OutPoint>>>>()?;
        Some(Geom::MultiContour(contours.into()))
    }
}

impl<P, C> CartesianGeometry2dSpecialization<P, MultiContourGeometryType> for C
where
    P: CartesianPoint2d,
    C: MultiContour
        + GeometryType<Type = MultiContourGeometryType, Space = CartesianSpace2d>
        + Geometry<Point = P>,
    C::Contour: Contour<Point = P> + CartesianGeometry2d<P>,
{
    fn is_point_inside_spec<Other: CartesianPoint2d<Num = P::Num>>(
        &self,
        _point: &Other,
        _tolerance: P::Num,
    ) -> bool {
        todo!()
    }

    fn bounding_rectangle_spec(&self) -> Option<Rect<P::Num>> {
        self.contours()
            .filter_map(|c| c.bounding_rectangle())
            .collect()
    }
}
