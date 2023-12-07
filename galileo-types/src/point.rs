use crate::geometry::GeometryMarker;
use crate::traits::PointMarker;
use crate::vec::Vec2d;
use crate::CartesianPoint2d;
use num_traits::{Bounded, Float};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Point2d<Num = f64> {
    x: Num,
    y: Num,
}

impl<Num: num_traits::Num + Copy> Point2d<Num> {
    pub const fn new(x: Num, y: Num) -> Self {
        Self { x, y }
    }

    pub fn x(&self) -> Num {
        self.x
    }
    pub fn y(&self) -> Num {
        self.y
    }

    pub fn multiply(&self, k: Num) -> Self {
        Self {
            x: self.x * k,
            y: self.y * k,
        }
    }
}

impl<Num: num_traits::Num + Copy> std::ops::Add<Point2d<Num>> for Point2d<Num> {
    type Output = Self;

    fn add(self, rhs: Point2d<Num>) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl<Num: Float> std::ops::Add<Vec2d<Num>> for Point2d<Num> {
    type Output = Self;

    fn add(self, rhs: Vec2d<Num>) -> Self::Output {
        Self {
            x: self.x + rhs.dx,
            y: self.y + rhs.dy,
        }
    }
}

impl<Num: Float> std::ops::Sub<Vec2d<Num>> for Point2d<Num> {
    type Output = Self;

    fn sub(self, rhs: Vec2d<Num>) -> Self::Output {
        Self {
            x: self.x - rhs.dx,
            y: self.y - rhs.dy,
        }
    }
}

impl<Num: Float> std::ops::Sub<Point2d<Num>> for Point2d<Num> {
    type Output = Vec2d<Num>;

    fn sub(self, rhs: Point2d<Num>) -> Self::Output {
        Vec2d {
            dx: self.x - rhs.x,
            dy: self.y - rhs.y,
        }
    }
}

impl<N> GeometryMarker for Point2d<N> {
    type Marker = PointMarker;
}

impl<Num: num_traits::Num + Copy + PartialOrd + Bounded> CartesianPoint2d for Point2d<Num> {
    type Num = Num;

    fn x(&self) -> Num {
        self.x
    }
    fn y(&self) -> Num {
        self.y
    }
}
