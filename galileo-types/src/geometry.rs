//! Abstract geometry types:
//! * [`Geometry`] trait for operations that are common for all geometry types
//! * [`CartesianGeometry2d`] for projected geometries.
//! * [`Geom`] enum that includes all geometry types to allow functions to operation on all of them

use crate::cartesian::{CartesianPoint2d, Rect};
use crate::geo::Projection;
use crate::geometry_type::{CartesianSpace2d, GeometryType, PointGeometryType};
use crate::impls::{Contour, MultiContour, MultiPoint, MultiPolygon, Polygon};

/// Enum of different geometry types. This enum implements the [`Geometry`] trait so you can use any generic geometry
/// method without knowing a specific geometry type you are working with.
pub enum Geom<P> {
    /// Point geometry.
    Point(P),
    /// MultiPoint geometry.
    MultiPoint(MultiPoint<P>),
    /// Contour geometry.
    Contour(Contour<P>),
    /// MultiContour geometry.
    MultiContour(MultiContour<P>),
    /// Polygon geometry.
    Polygon(Polygon<P>),
    /// MultiPolygon geometry.
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

/// Generic geometry.
///
/// This trait can be implemented manually for all geometry structs you use, or [`GeometryType`] trait can be used
/// for auto-implementation.
pub trait Geometry {
    /// Type of points this geometry consists of.
    type Point;
    /// Project the geometry using the given projection. Implementation of this method may choose to change type or
    /// properties of a geometry. For example a strait line in a projected CRS can be projected as curved line along
    /// the shortest path on the ellipsoid when projected into geographic coordinates.
    ///
    /// If the geometry cannot be projected with the given projection, `None` is returned.
    fn project<Proj>(&self, projection: &Proj) -> Option<Geom<Proj::OutPoint>>
    where
        Proj: Projection<InPoint = Self::Point> + ?Sized;
}

/// Geometry with cartesian *XY* coordinates.
pub trait CartesianGeometry2d<P: CartesianPoint2d>: Geometry<Point = P> {
    /// Checks if the given `point` is *inside* the geometry with the given tolerance.
    fn is_point_inside<Other: CartesianPoint2d<Num = P::Num>>(
        &self,
        point: &Other,
        tolerance: P::Num,
    ) -> bool;
    /// Returns bounding rectangle of the geometry.
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

/// This trait is used to automatically implement the [`Geometry`] trait using [`GeometryType`] trait.
pub trait GeometrySpecialization<GT, ST>: GeometryType {
    /// Type of the point of the geometry.
    type Point;

    /// See [`Geometry::project`].
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

/// This trait is used to automatically implement the [`CartesianGeometry2d`] trait using [`GeometryType`] trait.
pub trait CartesianGeometry2dSpecialization<P: CartesianPoint2d, GT>:
    GeometryType<Space = CartesianSpace2d> + Geometry<Point = P>
{
    /// See [`CartesianGeometry2d::is_point_inside`].
    fn is_point_inside_spec<Other: CartesianPoint2d<Num = P::Num>>(
        &self,
        point: &Other,
        tolerance: P::Num,
    ) -> bool;
    /// See [`CartesianGeometry2d::bounding_rectangle`].
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
