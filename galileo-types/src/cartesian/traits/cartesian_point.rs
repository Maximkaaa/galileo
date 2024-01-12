use crate::geo::traits::projection::Projection;
use crate::geometry::{Geom, GeometrySpecialization};
use crate::geometry_type::{CartesianSpace2d, GeometryType, PointGeometryType};
use crate::point::{CartesianPointType, Point, PointHelper};
use nalgebra::{Point2, Scalar, Vector2};
use num_traits::{Bounded, Float, FromPrimitive, Num};

pub trait CartesianPoint2d {
    type Num: Num + Copy + PartialOrd + Bounded + Scalar + FromPrimitive;

    fn x(&self) -> Self::Num;
    fn y(&self) -> Self::Num;

    fn equal(&self, other: &Self) -> bool
    where
        Self: Sized,
    {
        self.x() == other.x() && self.y() == other.y()
    }

    fn add(&self, vec: Vector2<Self::Num>) -> Point2<Self::Num>
    where
        Self: Sized,
    {
        Point2::new(self.x() + vec.x, self.y() + vec.y)
    }

    fn sub(&self, other: &impl CartesianPoint2d<Num = Self::Num>) -> Vector2<Self::Num> {
        Vector2::new(self.x() - other.x(), self.y() - other.y())
    }

    fn distance_sq(&self, other: &impl CartesianPoint2d<Num = Self::Num>) -> Self::Num {
        let v = self.sub(other);
        v.x * v.x + v.y * v.y
    }

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

pub trait CartesianPoint3d {
    type Num;

    fn x(&self) -> Self::Num;
    fn y(&self) -> Self::Num;
    fn z(&self) -> Self::Num;
}

impl<T> PointHelper<CartesianPointType> for T where
    T: CartesianPoint2d + Point<Type = CartesianPointType>
{
}

pub trait NewCartesianPoint2d<Num = f64>: CartesianPoint2d<Num = Num> {
    fn new(x: Num, y: Num) -> Self;
}

pub trait NewCartesianPoint3d<Num = f64>: CartesianPoint3d<Num = Num> {
    fn new(x: Num, y: Num, z: Num) -> Self;
}

pub trait CartesianPoint2dFloat<N: Float = f64>: CartesianPoint2d<Num = N> {
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

    fn project<Proj>(&self, projection: &Proj) -> Option<Geom<Proj::OutPoint>>
    where
        Proj: Projection<InPoint = Self::Point> + ?Sized,
    {
        Some(Geom::Point(projection.project(self)?))
    }
}
