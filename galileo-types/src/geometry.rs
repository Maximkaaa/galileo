use crate::cartesian::impls::contour::Contour;
use crate::cartesian::impls::multipolygon::MultiPolygon;
use crate::cartesian::impls::polygon::Polygon;
use crate::cartesian::rect::Rect;
use crate::cartesian::traits::cartesian_point::CartesianPoint2d;
use crate::geo::traits::projection::Projection;
use crate::geometry_type::GeometryType;
use crate::impls::multi_contour::MultiContour;
use crate::impls::multi_point::MultiPoint;

pub enum Geom<P> {
    Point(P),
    MultiPoint(MultiPoint<P>),
    Contour(Contour<P>),
    MultiContour(MultiContour<P>),
    Polygon(Polygon<P>),
    MultiPolygon(MultiPolygon<P>),
}

pub trait Geometry {
    type Point;
    fn project<Proj>(&self, projection: &Proj) -> Option<Geom<Proj::OutPoint>>
    where
        Proj: Projection<InPoint = Self::Point> + ?Sized;
}

pub trait CartesianGeometry2d<P: CartesianPoint2d>: Geometry<Point = P> {
    fn is_point_inside<Other: CartesianPoint2d<Num = P::Num>>(
        &self,
        point: &Other,
        tolerance: P::Num,
    ) -> bool;
    fn bounding_rectangle(&self) -> Rect<P::Num>;
}

impl<P> From<P> for Geom<P> {
    fn from(value: P) -> Self {
        Self::Point(value)
    }
}

impl<P> From<Contour<P>> for Geom<P> {
    fn from(value: Contour<P>) -> Self {
        Self::Contour(value)
    }
}

impl<P> From<Polygon<P>> for Geom<P> {
    fn from(value: Polygon<P>) -> Self {
        Self::Polygon(value)
    }
}

impl<P> From<MultiPolygon<P>> for Geom<P> {
    fn from(value: MultiPolygon<P>) -> Self {
        Self::MultiPolygon(value)
    }
}

pub trait GeometrySpecialization<GT, ST>: GeometryType {
    type Point;

    fn project<Proj>(&self, projection: &Proj) -> Option<Geom<Proj::OutPoint>>
    where
        Proj: Projection<InPoint = Self::Point> + ?Sized;
}

impl<T> Geometry for T
where
    T: GeometrySpecialization<<Self as GeometryType>::Type, <Self as GeometryType>::Space>,
{
    type Point = <Self as GeometrySpecialization<
        <Self as GeometryType>::Type,
        <Self as GeometryType>::Space,
    >>::Point;

    fn project<Proj>(&self, projection: &Proj) -> Option<Geom<Proj::OutPoint>>
    where
        Proj: Projection<InPoint = Self::Point> + ?Sized,
    {
        <Self as GeometrySpecialization<
            <Self as GeometryType>::Type,
            <Self as GeometryType>::Space,
        >>::project(self, projection)
    }
}
