use crate::geometry::GeometryMarker;
use crate::traits::contour::ContourMarker;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contour<Point> {
    pub points: Vec<Point>,
    pub is_closed: bool,
}

impl<Point> Contour<Point> {
    pub fn new(points: Vec<Point>, is_closed: bool) -> Self {
        Self { points, is_closed }
    }

    pub fn open(points: Vec<Point>) -> Self {
        Self {
            points,
            is_closed: false,
        }
    }

    pub fn closed(points: Vec<Point>) -> Self {
        Self {
            points,
            is_closed: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClosedContour<Point> {
    pub points: Vec<Point>,
}

impl<P> ClosedContour<P> {
    pub fn new(points: Vec<P>) -> Self {
        Self { points }
    }
}

impl<P> From<ClosedContour<P>> for Contour<P> {
    fn from(value: ClosedContour<P>) -> Self {
        Self {
            points: value.points,
            is_closed: true,
        }
    }
}

impl<P> GeometryMarker for Contour<P> {
    type Marker = ContourMarker;
}

impl<P> GeometryMarker for ClosedContour<P> {
    type Marker = ContourMarker;
}

impl<P> crate::traits::contour::ClosedContour for ClosedContour<P> {
    type Point = P;

    fn iter_points(&self) -> Box<dyn Iterator<Item = &'_ P> + '_> {
        Box::new(self.points.iter())
    }
}

// impl<'a, P> crate::traits::contour::ClosedContour<'a> for ClosedContour<P> {
//     fn iter_points(&'a self) -> Self::PointIterator {
//         todo!()
//     }
// }

impl<P> crate::traits::contour::Contour for Contour<P> {
    type Point = P;

    fn is_closed(&self) -> bool {
        self.is_closed
    }

    fn iter_points(&self) -> Box<dyn Iterator<Item = &P> + '_> {
        Box::new(self.points.iter())
    }
}

// impl<'a, Num: Float, P: CartesianPoint2d<Num = Num> + 'a> CartesianContour<'a, Num, P>
//     for Contour<P>
// {
// }
