use crate::cartesian::traits::cartesian_point::CartesianPoint2d;
use crate::contour::ClosedContour;
use crate::polygon::Polygon;
use crate::segment::Segment;
use nalgebra::Point2;

/// Polygon in 2d cartesian coordinates. This trait is auto-implemented for all illegible types.
pub trait CartesianPolygon {
    /// Type of the points of the polygon.
    type Point: CartesianPoint2d;

    /// Returns true if the `point` lies inside or on one of the polygon's sides.
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
                let ray_p1 = Point2::new(x, y);
                let ray_p2 = Point2::new(x_max, y);
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

#[cfg(test)]
mod tests {
    use crate::cartesian::impls::Point2d;
    use crate::cartesian::traits::polygon::*;

    #[test]
    fn contains_point() {
        let polygon = crate::impls::Polygon {
            outer_contour: crate::impls::ClosedContour {
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
