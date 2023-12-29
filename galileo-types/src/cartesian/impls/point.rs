use crate::cartesian::traits::cartesian_point::{CartesianPoint2d, NewCartesianPoint2d};
use crate::point::{CartesianPointType, Point};
pub use nalgebra::Point2;
use nalgebra::Scalar;
use num_traits::{Bounded, FromPrimitive};

pub type Point2d = Point2<f64>;

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
