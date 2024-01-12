use crate::contour::Contour;
use crate::geo::traits::projection::Projection;
use crate::geometry::{Geom, Geometry, GeometrySpecialization};
use crate::geometry_type::{GeometryType, MultiContourGeometryType};

pub trait MultiContour {
    type Contour: Contour;

    fn contours(&self) -> impl Iterator<Item = &Self::Contour>;
}

impl<C, Space> GeometrySpecialization<MultiContourGeometryType, Space> for C
where
    C: MultiContour + GeometryType<Type = MultiContourGeometryType, Space = Space>,
    C::Contour: Geometry,
{
    type Point = <C::Contour as Geometry>::Point;

    fn project<Proj>(&self, projection: &Proj) -> Option<Geom<Proj::OutPoint>>
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
            .collect::<Option<Vec<crate::cartesian::impls::contour::Contour<Proj::OutPoint>>>>()?;
        Some(Geom::MultiContour(contours.into()))
    }
}
