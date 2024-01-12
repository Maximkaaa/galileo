use crate::cartesian::traits::cartesian_point::CartesianPoint2d;
use crate::contour::{ClosedContour, Contour};
use num_traits::{One, Zero};
use std::cmp::Ordering;
use std::fmt::Debug;

pub trait CartesianClosedContour {
    type Point: CartesianPoint2d;

    fn area_signed(&self) -> <Self::Point as CartesianPoint2d>::Num
    where
        Self: Sized;
    fn winding(&self) -> Winding
    where
        Self: Sized;
}

impl<P, T> CartesianClosedContour for T
where
    P: CartesianPoint2d,
    T: ClosedContour<Point = P>,
{
    type Point = P;

    fn area_signed(&self) -> P::Num
    where
        Self: Sized,
    {
        let mut prev;
        let mut iter = self.iter_points_closing();
        if let Some(p) = iter.next() {
            prev = p;
        } else {
            return P::Num::zero();
        }

        let mut aggr = P::Num::zero();

        for p in iter {
            aggr = aggr + prev.x() * p.y() - p.x() * prev.y();
            prev = p;
        }

        aggr / (P::Num::one() + P::Num::one())
    }

    fn winding(&self) -> Winding
    where
        Self: Sized,
    {
        if self.area_signed() <= P::Num::zero() {
            Winding::Clockwise
        } else {
            Winding::CounterClockwise
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Winding {
    Clockwise,
    CounterClockwise,
}

pub trait CartesianContour<P: CartesianPoint2d>: Contour<Point = P> {
    fn distance_to_point_sq<Point>(&self, point: &Point) -> Option<P::Num>
    where
        Self: Sized,
        Point: CartesianPoint2d<Num = P::Num>,
    {
        self.iter_segments()
            .map(|v| v.distance_to_point_sq(point))
            .min_by(move |a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
    }
}

impl<T: Contour<Point = P>, P: CartesianPoint2d> CartesianContour<P> for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartesian::impls::contour::ClosedContour;
    use crate::cartesian::impls::point::Point2d;
    use crate::contour::Contour;

    #[test]
    fn iter_points_closing() {
        let contour = crate::cartesian::impls::contour::Contour::open(vec![
            Point2d::new(0.0, 0.0),
            Point2d::new(1.0, 1.0),
        ]);
        assert_eq!(contour.iter_points_closing().count(), 2);
        assert_eq!(
            *contour.iter_points_closing().last().unwrap(),
            Point2d::new(1.0, 1.0)
        );

        let contour = ClosedContour {
            points: vec![Point2d::new(0.0, 0.0), Point2d::new(1.0, 1.0)],
        };
        assert_eq!(contour.iter_points_closing().count(), 3);
        assert_eq!(
            *contour.iter_points_closing().last().unwrap(),
            Point2d::new(0.0, 0.0)
        );
    }

    #[test]
    fn iter_segments() {
        let contour = crate::cartesian::impls::contour::Contour::open(vec![Point2d::new(0.0, 0.0)]);
        assert_eq!(contour.iter_segments().count(), 0);

        let contour = crate::cartesian::impls::contour::Contour::open(vec![
            Point2d::new(0.0, 0.0),
            Point2d::new(1.0, 1.0),
        ]);
        assert_eq!(contour.iter_segments().count(), 1);
        assert_eq!(
            contour.iter_segments().last().unwrap(),
            Segment(&Point2d::new(0.0, 0.0), &Point2d::new(1.0, 1.0))
        );

        let contour = ClosedContour {
            points: vec![Point2d::new(0.0, 0.0), Point2d::new(1.0, 1.0)],
        };
        assert_eq!(contour.iter_segments().count(), 2);
        assert_eq!(
            contour.iter_segments().last().unwrap(),
            Segment(&Point2d::new(1.0, 1.0), &Point2d::new(0.0, 0.0))
        );
    }

    #[test]
    fn distance_to_point() {
        let contour = ClosedContour {
            points: vec![
                Point2d::new(0.0, 0.0),
                Point2d::new(1.0, 1.0),
                Point2d::new(1.0, 0.0),
            ],
        };

        assert_eq!(
            contour.distance_to_point_sq(&Point2d::new(0.0, 0.0)),
            Some(0.0)
        );
        assert_eq!(
            contour.distance_to_point_sq(&Point2d::new(0.5, 0.0)),
            Some(0.0)
        );
        assert_eq!(
            contour.distance_to_point_sq(&Point2d::new(0.5, 0.5)),
            Some(0.0)
        );
        assert_eq!(
            contour.distance_to_point_sq(&Point2d::new(0.0, 1.0)),
            Some(0.5)
        );
        assert_eq!(
            contour.distance_to_point_sq(&Point2d::new(2.0, 2.0)),
            Some(2.0)
        );
        assert_eq!(
            contour.distance_to_point_sq(&Point2d::new(-2.0, -2.0)),
            Some(8.0)
        );
    }

    #[test]
    fn area() {
        let contour = ClosedContour::new(vec![
            Point2d::new(0.0, 0.0),
            Point2d::new(0.0, 1.0),
            Point2d::new(1.0, 0.0),
        ]);

        assert_eq!(contour.area_signed(), -0.5);

        let contour = ClosedContour::new(vec![
            Point2d::new(0.0, 0.0),
            Point2d::new(1.0, 0.0),
            Point2d::new(0.0, 1.0),
        ]);

        assert_eq!(contour.area_signed(), 0.5);
    }

    #[test]
    fn winding() {
        let contour = ClosedContour::new(vec![
            Point2d::new(0.0, 0.0),
            Point2d::new(0.0, 1.0),
            Point2d::new(1.0, 0.0),
        ]);

        assert_eq!(contour.winding(), Winding::Clockwise);

        let contour = ClosedContour::new(vec![
            Point2d::new(0.0, 0.0),
            Point2d::new(1.0, 0.0),
            Point2d::new(0.0, 1.0),
        ]);

        assert_eq!(contour.winding(), Winding::CounterClockwise);
    }
}
