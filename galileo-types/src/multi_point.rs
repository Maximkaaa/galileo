use crate::geo::traits::projection::Projection;
use crate::geometry::{Geom, GeometrySpecialization};
use crate::geometry_type::{GeometryType, MultiPointGeometryType};

pub trait MultiPoint {
    type Point;

    fn iter_points(&self) -> impl Iterator<Item = &'_ Self::Point>;
}

impl<P, Space> GeometrySpecialization<MultiPointGeometryType, Space> for P
where
    P: MultiPoint + GeometryType<Type = MultiPointGeometryType, Space = Space>,
{
    type Point = P::Point;

    fn project<Proj>(&self, projection: &Proj) -> Option<Geom<Proj::OutPoint>>
    where
        Proj: Projection<InPoint = Self::Point> + ?Sized,
    {
        let points = self
            .iter_points()
            .map(|p| projection.project(p))
            .collect::<Option<Vec<Proj::OutPoint>>>()?;
        Some(Geom::MultiPoint(points.into()))
    }
}
