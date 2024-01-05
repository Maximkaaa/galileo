use crate::cartesian::traits::cartesian_point::{
    CartesianPoint2d, CartesianPoint3d, NewCartesianPoint2d, NewCartesianPoint3d,
};
use crate::geo::traits::projection::Projection;
use crate::geometry::{Geom, Geometry};
use crate::point::{CartesianPointType, Point};
pub use nalgebra::Point2;
use nalgebra::{Point3, Scalar};
use num_traits::{Bounded, FromPrimitive};

pub type Point2d = Point2<f64>;
pub type Point3d = Point3<f64>;

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
        Point2::new(x, y)
    }
}

impl<Num: Scalar> Point for Point2<Num> {
    type Type = CartesianPointType;
    type Num = Num;
    const DIMENSIONS: usize = 2;
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
        Point3::new(x, y, z)
    }
}

impl<Num: Scalar> Geometry for Point2<Num> {
    type Point = Point2<Num>;

    fn project<P: Projection<InPoint = Self::Point> + ?Sized>(
        &self,
        projection: &P,
    ) -> Option<Geom<P::OutPoint>> {
        Some(Geom::Point(projection.project(self)?))
    }
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
