use crate::cartesian::impls::contour::Contour;
use crate::cartesian::impls::multipolygon::MultiPolygon;
use crate::cartesian::impls::polygon::Polygon;
use crate::cartesian::rect::Rect;
use crate::cartesian::traits::cartesian_point::CartesianPoint2d;
use crate::geo::traits::projection::Projection;
use crate::geometry_type::{CartesianSpace2d, GeometryType, PointGeometryType};
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

impl<P: GeometryType> Geometry for Geom<P> {
    type Point = P;

    fn project<Proj>(&self, projection: &Proj) -> Option<Geom<Proj::OutPoint>>
    where
        Proj: Projection<InPoint = <Self as Geometry>::Point> + ?Sized,
    {
        match &self {
            Geom::Point(v) => Some(Geom::Point(projection.project(v)?)),
            Geom::MultiPoint(v) => v.project(projection),
            Geom::Contour(v) => v.project(projection),
            Geom::MultiContour(v) => v.project(projection),
            Geom::Polygon(v) => v.project(projection),
            Geom::MultiPolygon(v) => v.project(projection),
        }
    }
}

impl<P> CartesianGeometry2d<P> for Geom<P>
where
    P: CartesianPoint2d + GeometryType<Type = PointGeometryType, Space = CartesianSpace2d>,
{
    fn is_point_inside<Other: CartesianPoint2d<Num = P::Num>>(
        &self,
        point: &Other,
        tolerance: P::Num,
    ) -> bool {
        match self {
            Geom::Point(v) => v.is_point_inside(point, tolerance),
            Geom::MultiPoint(v) => v.is_point_inside(point, tolerance),
            Geom::Contour(v) => v.is_point_inside(point, tolerance),
            Geom::MultiContour(v) => v.is_point_inside(point, tolerance),
            Geom::Polygon(v) => v.is_point_inside(point, tolerance),
            Geom::MultiPolygon(v) => v.is_point_inside(point, tolerance),
        }
    }

    fn bounding_rectangle(&self) -> Option<Rect<P::Num>> {
        match self {
            Geom::Point(v) => v.bounding_rectangle(),
            Geom::MultiPoint(v) => v.bounding_rectangle(),
            Geom::Contour(v) => v.bounding_rectangle(),
            Geom::MultiContour(v) => v.bounding_rectangle(),
            Geom::Polygon(v) => v.bounding_rectangle(),
            Geom::MultiPolygon(v) => v.bounding_rectangle(),
        }
    }
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
    fn bounding_rectangle(&self) -> Option<Rect<P::Num>>;
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

    fn project_spec<Proj>(&self, projection: &Proj) -> Option<Geom<Proj::OutPoint>>
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
        >>::project_spec(self, projection)
    }
}

pub trait CartesianGeometry2dSpecialization<P: CartesianPoint2d, GT>:
    GeometryType<Space = CartesianSpace2d> + Geometry<Point = P>
{
    fn is_point_inside_spec<Other: CartesianPoint2d<Num = P::Num>>(
        &self,
        point: &Other,
        tolerance: P::Num,
    ) -> bool;
    fn bounding_rectangle_spec(&self) -> Option<Rect<P::Num>>;
}

impl<P, T> CartesianGeometry2d<P> for T
where
    P: CartesianPoint2d,
    T: CartesianGeometry2dSpecialization<P, <Self as GeometryType>::Type>,
{
    fn is_point_inside<Other: CartesianPoint2d<Num = P::Num>>(
        &self,
        point: &Other,
        tolerance: P::Num,
    ) -> bool {
        self.is_point_inside_spec(point, tolerance)
    }

    fn bounding_rectangle(&self) -> Option<Rect<P::Num>> {
        self.bounding_rectangle_spec()
    }
}
