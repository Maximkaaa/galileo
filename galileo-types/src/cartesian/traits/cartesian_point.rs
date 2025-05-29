use nalgebra::Scalar;
use num_traits::{Bounded, Float, FromPrimitive, Num};

use crate::cartesian::{Point2, Rect, Vector2};
use crate::geo::Projection;
use crate::geometry::{CartesianGeometry2dSpecialization, Geom, GeometrySpecialization};
use crate::geometry_type::{CartesianSpace2d, GeometryType, PointGeometryType};

/// Point in a 2d cartesian space.
///
/// Only two methods require an implementation to get access all the methods of this trait:
///
/// ```
/// use galileo_types::cartesian::CartesianPoint2d;
///
/// struct MyPoint(f64, f64);
///
/// impl CartesianPoint2d for MyPoint {
///     type Num = f64;
///     fn x(&self) -> Self::Num { self.0 }
///     fn y(&self) -> Self::Num { self.1 }
/// }
/// ```
pub trait CartesianPoint2d {
    /// Number type used for coordinates.
    type Num: Num + Copy + PartialOrd + Bounded + Scalar + FromPrimitive;

    /// First coordinate.
    fn x(&self) -> Self::Num;
    /// Second coordinate.
    fn y(&self) -> Self::Num;

    /// Returns true, if both *x* and *y* of two points are exactly equal.
    fn equal(&self, other: &Self) -> bool
    where
        Self: Sized,
    {
        self.x() == other.x() && self.y() == other.y()
    }

    /// Moves the point by the `vec`.
    fn add(&self, vec: Vector2<Self::Num>) -> Point2<Self::Num>
    where
        Self: Sized,
    {
        Point2::new(self.x() + vec.dx(), self.y() + vec.dy())
    }

    /// Returns a vector between this and the `other` points.
    fn sub(&self, other: &impl CartesianPoint2d<Num = Self::Num>) -> Vector2<Self::Num> {
        Vector2::new(self.x() - other.x(), self.y() - other.y())
    }

    /// Returns squared euclidean distance between two points.
    fn distance_sq(&self, other: &impl CartesianPoint2d<Num = Self::Num>) -> Self::Num {
        let v = self.sub(other);
        v.dx() * v.dx() + v.dy() * v.dy()
    }

    /// Returns [taxicab distance](https://en.wikipedia.org/wiki/Taxicab_geometry) between two points.
    fn taxicab_distance(&self, other: &impl CartesianPoint2d<Num = Self::Num>) -> Self::Num {
        let dx = if self.x() >= other.x() {
            self.x() - other.x()
        } else {
            other.x() - self.x()
        };
        let dy = if self.y() >= other.y() {
            self.y() - other.y()
        } else {
            other.y() - self.y()
        };

        dx + dy
    }
}

/// Point in a 3d cartesian space.
pub trait CartesianPoint3d {
    /// Number type used for coordinates.
    type Num;

    /// First coordinate.
    fn x(&self) -> Self::Num;
    /// Second coordinate.
    fn y(&self) -> Self::Num;
    /// Third coordinate.
    fn z(&self) -> Self::Num;
}

/// A 2d cartesian point that can be constructed from only the coordinates.
pub trait NewCartesianPoint2d<Num = f64>: CartesianPoint2d<Num = Num> {
    /// Creates a new point with the given coordinates.
    fn new(x: Num, y: Num) -> Self;
}

/// A 3d cartesian point that can be constructed from only the coordinates.
pub trait NewCartesianPoint3d<Num = f64>: CartesianPoint3d<Num = Num> {
    /// Creates a new point with the given coordinates.
    fn new(x: Num, y: Num, z: Num) -> Self;
}

/// Methods that apply only when a point has float-type coordinates.
pub trait CartesianPoint2dFloat<N: Float = f64>: CartesianPoint2d<Num = N> {
    /// Euclidean distance between two points.
    fn distance(&self, other: &impl CartesianPoint2d<Num = N>) -> N {
        self.distance_sq(other).sqrt()
    }
}

impl<N: Float, T: CartesianPoint2d<Num = N>> CartesianPoint2dFloat<N> for T {}

impl<P> GeometrySpecialization<PointGeometryType, CartesianSpace2d> for P
where
    P: CartesianPoint2d + GeometryType<Type = PointGeometryType, Space = CartesianSpace2d>,
{
    type Point = P;

    fn project_spec<Proj>(&self, projection: &Proj) -> Option<Geom<Proj::OutPoint>>
    where
        Proj: Projection<InPoint = Self::Point> + ?Sized,
    {
        Some(Geom::Point(projection.project(self)?))
    }
}

impl<P> CartesianGeometry2dSpecialization<P, PointGeometryType> for P
where
    P: CartesianPoint2d + GeometryType<Type = PointGeometryType, Space = CartesianSpace2d> + Copy,
{
    fn is_point_inside_spec<Other: CartesianPoint2d<Num = P::Num>>(
        &self,
        point: &Other,
        tolerance: P::Num,
    ) -> bool {
        self.distance_sq(point) < tolerance * tolerance
    }

    fn bounding_rectangle_spec(&self) -> Option<Rect<P::Num>> {
        Some(Rect::from_point(self))
    }
}
