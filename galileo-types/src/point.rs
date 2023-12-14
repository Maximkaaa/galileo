use crate::geometry::GeometryMarker;
use crate::traits::PointMarker;
use crate::CartesianPoint2d;
pub use nalgebra::Point2;
use nalgebra::Scalar;
use num_traits::{Bounded, FromPrimitive};

pub type Point2d = Point2<f64>;

impl<N: Scalar> GeometryMarker for Point2<N> {
    type Marker = PointMarker;
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
