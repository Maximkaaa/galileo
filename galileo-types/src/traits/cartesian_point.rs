use crate::geometry::{
    CartesianPointType, Geometry, GeometryHelper, GeometryMarker, Point, PointHelper,
};
use crate::rect::Rect;
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

impl<T> PointHelper<CartesianPointType> for T where
    T: CartesianPoint2d + Point<Type = CartesianPointType>
{
}

pub trait NewCartesianPoint2d<Num = f64>: CartesianPoint2d<Num = Num> {
    fn new(x: Num, y: Num) -> Self;
}

pub trait CartesianPoint2dFloat<N: Float = f64>: CartesianPoint2d<Num = N> {
    fn distance(&self, other: &impl CartesianPoint2d<Num = N>) -> N {
        self.distance_sq(other).sqrt()
    }
}

impl<N: Float, T: CartesianPoint2d<Num = N>> CartesianPoint2dFloat<N> for T {}

pub struct PointMarker {}

impl<N, T> GeometryHelper<PointMarker> for T
where
    N: Num + Copy + PartialOrd + Bounded + Scalar + FromPrimitive,
    T: CartesianPoint2d<Num = N>
        + GeometryMarker<Marker = PointMarker>
        + Point<Type = CartesianPointType, Num = N>
        + Geometry<Point = Self>,
{
    type Point = T;

    fn __bounding_rect(&self) -> Rect<N> {
        Rect::from_point(self)
    }

    fn __contains_point<P>(&self, point: &P, tolerance: N) -> bool
    where
        P: CartesianPoint2d<Num = N>,
    {
        self.distance_sq(point) < tolerance * tolerance
    }
}
