use crate::cartesian::{CartesianPoint2d, Rect};
use crate::geo::Projection;
use crate::geometry::{
    CartesianGeometry2d, CartesianGeometry2dSpecialization, Geom, GeometrySpecialization,
};
use crate::geometry_type::{CartesianSpace2d, GeometryType, MultiPointGeometryType};

/// Geometry type consisting of several points.
pub trait MultiPoint {
    /// Point type.
    type Point;

    /// Iterates over points.
    fn iter_points(&self) -> impl Iterator<Item = &'_ Self::Point>;
}

impl<P, Space> GeometrySpecialization<MultiPointGeometryType, Space> for P
where
    P: MultiPoint + GeometryType<Type = MultiPointGeometryType, Space = Space>,
{
    type Point = P::Point;

    fn project_spec<Proj>(&self, projection: &Proj) -> Option<Geom<Proj::OutPoint>>
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

impl<P> CartesianGeometry2dSpecialization<P::Point, MultiPointGeometryType> for P
where
    P: MultiPoint + GeometryType<Type = MultiPointGeometryType, Space = CartesianSpace2d>,
    P::Point: CartesianPoint2d + CartesianGeometry2d<P::Point>,
{
    fn is_point_inside_spec<Other: CartesianPoint2d<Num = <P::Point as CartesianPoint2d>::Num>>(
        &self,
        point: &Other,
        tolerance: <P::Point as CartesianPoint2d>::Num,
    ) -> bool {
        self.iter_points()
            .any(|p| p.is_point_inside(point, tolerance))
    }

    fn bounding_rectangle_spec(&self) -> Option<Rect<<P::Point as CartesianPoint2d>::Num>> {
        Rect::from_points(self.iter_points())
    }
}
