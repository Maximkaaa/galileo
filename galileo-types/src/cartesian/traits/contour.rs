use crate::cartesian::traits::cartesian_point::CartesianPoint2d;
use crate::segment::Segment;
use num_traits::{One, Zero};
use std::cmp::Ordering;
use std::fmt::Debug;

pub trait Contour {
    type Point;

    fn is_closed(&self) -> bool;

    fn iter_points(&self) -> Box<dyn Iterator<Item = &'_ Self::Point> + '_>;

    fn iter_points_closing(&self) -> Box<dyn Iterator<Item = &Self::Point> + '_>
    where
        Self: Sized,
    {
        Box::new(ContourPointsIterator::new(
            self.iter_points(),
            self.is_closed(),
        ))
    }

    fn iter_segments(&self) -> Box<dyn Iterator<Item = Segment<'_, Self::Point>> + '_>
    where
        Self: Sized,
    {
        Box::new(ContourSegmentIterator::new(ContourPointsIterator::new(
            self.iter_points(),
            self.is_closed(),
        )))
    }
}

pub trait ClosedContour {
    type Point;
    fn iter_points(&self) -> Box<dyn Iterator<Item = &'_ Self::Point> + '_>;
}

impl<P, T: ClosedContour<Point = P>> Contour for T {
    type Point = P;

    fn is_closed(&self) -> bool {
        true
    }

    fn iter_points(&self) -> Box<dyn Iterator<Item = &'_ Self::Point> + '_> {
        self.iter_points()
    }
}

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

pub struct ContourPointsIterator<'a, P> {
    points_iter: Box<dyn Iterator<Item = &'a P> + 'a>,
    is_closed: bool,
    first_point: Option<&'a P>,
}

impl<'a, P: 'a> ContourPointsIterator<'a, P> {
    fn new(points_iter: Box<dyn Iterator<Item = &'a P> + 'a>, is_closed: bool) -> Self {
        Self {
            points_iter,
            is_closed,
            first_point: None,
        }
    }
}

impl<'a, P> Iterator for ContourPointsIterator<'a, P> {
    type Item = &'a P;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.points_iter.next();
        if self.is_closed && self.first_point.is_none() {
            self.first_point = next;
        }

        if next.is_none() {
            self.first_point.take()
        } else {
            next
        }
    }
}

pub struct ContourSegmentIterator<'a, P: 'a> {
    points_iter: ContourPointsIterator<'a, P>,
    prev_point: Option<&'a P>,
}

impl<'a, P> ContourSegmentIterator<'a, P> {
    fn new(points_iter: ContourPointsIterator<'a, P>) -> Self {
        Self {
            points_iter,
            prev_point: None,
        }
    }
}

impl<'a, P> Iterator for ContourSegmentIterator<'a, P> {
    type Item = Segment<'a, P>;

    fn next(&mut self) -> Option<Self::Item> {
        let next_point = self.points_iter.next()?;
        let prev_point = self.prev_point.replace(next_point);

        match prev_point {
            Some(prev) => Some(Segment(prev, next_point)),
            None => self.next(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartesian::impls::contour::ClosedContour;
    use crate::cartesian::impls::point::Point2d;

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
