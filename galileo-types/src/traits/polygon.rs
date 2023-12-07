use crate::bounding_rect::BoundingRect;
use crate::geometry::{Geometry, GeometryHelper, GeometryMarker};
use crate::segment::Segment;
use crate::traits::contour::{ClosedContour, Contour};
use crate::{CartesianPoint2d, Point2d};

pub trait Polygon: GeometryMarker {
    type Contour: ClosedContour;

    fn outer_contour(&self) -> &Self::Contour;
    fn inner_contours(&self) -> Box<dyn Iterator<Item = &'_ Self::Contour> + '_>;

    fn iter_contours(&self) -> Box<dyn Iterator<Item = &'_ Self::Contour> + '_> {
        Box::new(std::iter::once(self.outer_contour()).chain(self.inner_contours()))
    }

    fn iter_segments(
        &self,
    ) -> Box<dyn Iterator<Item = Segment<'_, <Self::Contour as Contour>::Point>> + '_> {
        Box::new(self.iter_contours().flat_map(Self::Contour::iter_segments))
    }
}

pub trait CartesianPolygon {
    type Point: CartesianPoint2d;

    fn contains_point<P>(&self, point: &P) -> bool
    where
        P: CartesianPoint2d<Num = <Self::Point as CartesianPoint2d>::Num>;
}

impl<P, C, T> CartesianPolygon for T
where
    P: CartesianPoint2d,
    C: ClosedContour<Point = P>,
    T: Polygon<Contour = C>,
{
    type Point = P;

    fn contains_point<Point: CartesianPoint2d<Num = P::Num>>(&self, point: &Point) -> bool {
        let mut wn = 0i64;
        let x = point.x();
        let y = point.y();

        for segment in self.iter_segments() {
            if segment.0.x() < x && segment.1.x() < x {
                continue;
            }

            let is_to_right = segment.0.x() > x && segment.1.x() > x || {
                let x_max = if segment.0.x() > segment.1.x() {
                    segment.0.x()
                } else {
                    segment.1.x()
                };
                let ray_p1 = Point2d::new(x, y);
                let ray_p2 = Point2d::new(x_max, y);
                let ray = Segment(&ray_p1, &ray_p2);

                segment.intersects(&ray)
            };

            if is_to_right {
                if segment.0.y() < y && segment.1.y() >= y {
                    wn += 1;
                } else if segment.0.y() > y && segment.1.y() <= y {
                    wn -= 1;
                }
            }
        }

        wn != 0
    }
}

pub struct PolygonMarker {}

impl<P, C, T> GeometryHelper<PolygonMarker> for T
where
    P: CartesianPoint2d,
    C: ClosedContour<Point = P>,
    T: Polygon<Contour = C> + GeometryMarker<Marker = PolygonMarker>,
{
    type Num = P::Num;

    fn __bounding_rect(&self) -> BoundingRect<Self::Num> {
        self.outer_contour().bounding_rect()
    }

    fn __contains_point<Point>(&self, point: &Point, _: Self::Num) -> bool
    where
        Point: CartesianPoint2d<Num = Self::Num>,
    {
        self.contains_point(point)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ClosedContour, Point2d};

    #[test]
    fn contains_point() {
        let polygon = crate::Polygon {
            outer_contour: ClosedContour {
                points: vec![
                    Point2d::new(0.0, 0.0),
                    Point2d::new(1.0, 1.0),
                    Point2d::new(1.0, 0.0),
                ],
            },
            inner_contours: vec![],
        };

        assert!(polygon.contains_point(&Point2d::new(0.0, 0.0)));
        assert!(polygon.contains_point(&Point2d::new(1.0, 1.0)));
        assert!(polygon.contains_point(&Point2d::new(0.5, 0.0)));
        assert!(polygon.contains_point(&Point2d::new(0.2, 0.1)));
        assert!(!polygon.contains_point(&Point2d::new(0.2, 0.3)));
        assert!(!polygon.contains_point(&Point2d::new(0.2, -0.3)));
        assert!(!polygon.contains_point(&Point2d::new(1.1, 0.0)));
    }
}
