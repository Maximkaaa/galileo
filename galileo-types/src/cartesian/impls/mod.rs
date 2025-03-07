use approx::AbsDiffEq;
use nalgebra::Scalar;
use num_traits::{Bounded, FromPrimitive};
use serde::{Deserialize, Serialize};

use crate::cartesian::traits::{
    CartesianPoint2d, CartesianPoint3d, NewCartesianPoint2d, NewCartesianPoint3d,
};
use crate::geo::Projection;
use crate::geometry::{Geom, Geometry};
use crate::geometry_type::{CartesianSpace2d, GeometryType, PointGeometryType};

/// A point in 2-dimensional cartesian coordinate space.
#[derive(Debug, Default, Copy, Clone, PartialEq, Hash, Serialize, Deserialize)]
pub struct Point2<Num = f64> {
    x: Num,
    y: Num,
}

impl<Num> Point2<Num> {
    /// Creates a new point with the given coordinates.
    pub const fn new(x: Num, y: Num) -> Self {
        Self { x, y }
    }

    /// Returns coordinates of the point as an array of `Num`.
    pub fn coords(&self) -> [Num; 2]
    where
        Num: Copy,
    {
        [self.x, self.y]
    }
}

/// Vector between two points in 2-dimensional cartesian coordinate space.
#[derive(Debug, Default, Copy, Clone, PartialEq, Hash, Serialize, Deserialize)]
pub struct Vector2<Num = f64> {
    dx: Num,
    dy: Num,
}

impl<Num: Copy> Vector2<Num> {
    /// Creates a new vector with the given coordinates.
    pub fn new(dx: Num, dy: Num) -> Self {
        Self { dx, dy }
    }

    /// Returns x coordinate of the vector.
    pub fn dx(&self) -> Num {
        self.dx
    }

    /// Returns y coordinate of the vector.
    pub fn dy(&self) -> Num {
        self.dy
    }

    /// Updates x coordinate of the vector.
    pub fn set_dx(&mut self, dx: Num) {
        self.dx = dx;
    }

    /// Updates y coordinate of the vector.
    pub fn set_dy(&mut self, dy: Num) {
        self.dy = dy;
    }

    /// Returns squared magnitude (squared length) of the vector.
    pub fn magnitude_sq(&self) -> Num
    where
        Num: num_traits::Num,
    {
        self.dx * self.dx + self.dy * self.dy
    }

    /// Returns magnitude (length) of the vector.
    pub fn magnitude(&self) -> Num
    where
        Num: num_traits::Float,
    {
        self.magnitude_sq().sqrt()
    }
}

impl<Num> std::ops::Sub<Point2<Num>> for Point2<Num>
where
    Num: std::ops::Sub<Num, Output = Num>,
{
    type Output = Vector2<Num>;

    fn sub(self, rhs: Point2<Num>) -> Self::Output {
        Vector2 {
            dx: self.x - rhs.x,
            dy: self.y - rhs.y,
        }
    }
}

impl<Num> std::ops::Add<Vector2<Num>> for Point2<Num>
where
    Num: std::ops::Add<Num, Output = Num>,
{
    type Output = Point2<Num>;

    fn add(self, rhs: Vector2<Num>) -> Self::Output {
        Self {
            x: self.x + rhs.dx,
            y: self.y + rhs.dy,
        }
    }
}

impl<Num> std::ops::Sub<Vector2<Num>> for Point2<Num>
where
    Num: std::ops::Sub<Num, Output = Num>,
{
    type Output = Point2<Num>;

    fn sub(self, rhs: Vector2<Num>) -> Self::Output {
        Self {
            x: self.x - rhs.dx,
            y: self.y - rhs.dy,
        }
    }
}

impl<Num> std::ops::Mul<Num> for Vector2<Num>
where
    Num: std::ops::Mul<Num, Output = Num> + Copy,
{
    type Output = Vector2<Num>;

    fn mul(self, rhs: Num) -> Self::Output {
        Self {
            dx: self.dx * rhs,
            dy: self.dy * rhs,
        }
    }
}

impl<Num> AbsDiffEq for Point2<Num>
where
    Num: AbsDiffEq<Num, Epsilon = Num> + Copy,
{
    type Epsilon = Num;

    fn default_epsilon() -> Self::Epsilon {
        Num::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.x.abs_diff_eq(&other.x, epsilon) && self.y.abs_diff_eq(&other.y, epsilon)
    }
}

/// A point in 3-dimensional cartesian coordinate space.
#[derive(Debug, Default, Copy, Clone, PartialEq, Hash, Serialize, Deserialize)]
pub struct Point3<Num = f64> {
    x: Num,
    y: Num,
    z: Num,
}

impl<Num> Point3<Num> {
    /// Creates a new instance of the point by its coordinates.
    pub const fn new(x: Num, y: Num, z: Num) -> Self {
        Self { x, y, z }
    }
}

/// Vector between two points in 3-dimensional cartesian coordinate space.
#[derive(Debug, Default, Copy, Clone, PartialEq, Hash, Serialize, Deserialize)]
pub struct Vector3<Num = f64> {
    dx: Num,
    dy: Num,
    dz: Num,
}

impl<Num: Copy> Vector3<Num> {
    /// Creates a new vector with the given coordinates.
    pub fn new(dx: Num, dy: Num, dz: Num) -> Self {
        Self { dx, dy, dz }
    }

    /// Returns x coordinate of the vector.
    pub fn dx(&self) -> Num {
        self.dx
    }

    /// Returns y coordinate of the vector.
    pub fn dy(&self) -> Num {
        self.dy
    }

    /// Returns z coordinate of the vector.
    pub fn dz(&self) -> Num {
        self.dz
    }

    /// Updates x coordinate of the vector.
    pub fn set_dx(&mut self, dx: Num) {
        self.dx = dx;
    }

    /// Updates y coordinate of the vector.
    pub fn set_dy(&mut self, dy: Num) {
        self.dy = dy;
    }

    /// Updates z coordinate of the vector.
    pub fn set_dz(&mut self, dz: Num) {
        self.dz = dz;
    }
}

impl<Num> std::ops::Sub<Point3<Num>> for Point3<Num>
where
    Num: std::ops::Sub<Num, Output = Num>,
{
    type Output = Vector3<Num>;

    fn sub(self, rhs: Point3<Num>) -> Self::Output {
        Vector3 {
            dx: self.x - rhs.x,
            dy: self.y - rhs.y,
            dz: self.z - rhs.z,
        }
    }
}

impl<Num> std::ops::Add<Vector3<Num>> for Point3<Num>
where
    Num: std::ops::Add<Num, Output = Num>,
{
    type Output = Point3<Num>;

    fn add(self, rhs: Vector3<Num>) -> Self::Output {
        Self {
            x: self.x + rhs.dx,
            y: self.y + rhs.dy,
            z: self.z + rhs.dz,
        }
    }
}

impl<Num> std::ops::Sub<Vector3<Num>> for Point3<Num>
where
    Num: std::ops::Sub<Num, Output = Num>,
{
    type Output = Point3<Num>;

    fn sub(self, rhs: Vector3<Num>) -> Self::Output {
        Self {
            x: self.x - rhs.dx,
            y: self.y - rhs.dy,
            z: self.z - rhs.dz,
        }
    }
}

impl<Num> std::ops::Mul<Num> for Vector3<Num>
where
    Num: std::ops::Mul<Num, Output = Num> + Copy,
{
    type Output = Vector3<Num>;

    fn mul(self, rhs: Num) -> Self::Output {
        Self {
            dx: self.dx * rhs,
            dy: self.dy * rhs,
            dz: self.dz * rhs,
        }
    }
}

impl<Num: num_traits::Num + Copy + PartialOrd + Bounded + Scalar + FromPrimitive> CartesianPoint2d
    for Point2<Num>
{
    type Num = Num;

    fn x(&self) -> Num {
        self.x
    }
    fn y(&self) -> Num {
        self.y
    }
}

impl<Num: num_traits::Num + Copy + PartialOrd + Bounded + Scalar + FromPrimitive>
    NewCartesianPoint2d<Num> for Point2<Num>
{
    fn new(x: Num, y: Num) -> Self {
        Point2 { x, y }
    }
}

impl<Num: Scalar + Copy> CartesianPoint3d for Point3<Num> {
    type Num = Num;

    fn x(&self) -> Self::Num {
        self.x
    }

    fn y(&self) -> Self::Num {
        self.y
    }

    fn z(&self) -> Self::Num {
        self.z
    }
}

impl<Num: Scalar + Copy> NewCartesianPoint3d<Num> for Point3<Num> {
    fn new(x: Num, y: Num, z: Num) -> Self {
        Point3 { x, y, z }
    }
}

impl<Num: Scalar> GeometryType for Point2<Num> {
    type Type = PointGeometryType;
    type Space = CartesianSpace2d;
}

impl<Num: Scalar> Geometry for Point3<Num> {
    type Point = Point3<Num>;

    fn project<P: Projection<InPoint = Self::Point> + ?Sized>(
        &self,
        projection: &P,
    ) -> Option<Geom<P::OutPoint>> {
        Some(Geom::Point(projection.project(self)?))
    }
}
