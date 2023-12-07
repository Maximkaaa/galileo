use crate::orient::Orientation;
use crate::CartesianPoint2d;
use num_traits::{One, Zero};

#[derive(Debug, PartialEq)]
pub struct Segment<'a, Point>(pub &'a Point, pub &'a Point);

impl<'a, P: CartesianPoint2d> Segment<'a, P> {
    pub fn distance_to_point_sq<Point: CartesianPoint2d<Num = P::Num>>(
        &self,
        point: &Point,
    ) -> P::Num {
        if self.0.equal(&self.1) {
            return self.0.distance_sq(point);
        }

        let ds = self.1.sub(self.0);
        let dp = point.sub(self.0);
        let ds_len = ds.length_sq();

        let r = (dp.dx * ds.dx + dp.dy * ds.dy) / ds_len;
        if r <= P::Num::zero() {
            self.0.distance_sq(point)
        } else if r >= P::Num::one() {
            self.1.distance_sq(point)
        } else {
            let s = (dp.dy * ds.dx - dp.dx * ds.dy) / ds_len;
            (s * s) * ds_len
        }
    }

    pub fn intersects<Point: CartesianPoint2d<Num = P::Num>>(
        &self,
        other: &Segment<Point>,
    ) -> bool {
        fn on_segment<Num: num_traits::Num + PartialOrd>(
            p: &impl CartesianPoint2d<Num = Num>,
            q: &impl CartesianPoint2d<Num = Num>,
            r: &impl CartesianPoint2d<Num = Num>,
        ) -> bool {
            let x_max = if p.x() >= r.x() { p.x() } else { r.x() };
            let x_min = if p.x() <= r.x() { p.x() } else { r.x() };
            let y_max = if p.y() >= r.y() { p.x() } else { r.x() };
            let y_min = if p.y() <= r.y() { p.x() } else { r.x() };

            q.x() <= x_max && q.x() >= x_min && q.y() <= y_max && q.y() >= y_min
        }

        let o1 = Orientation::triplet(self.0, other.0, self.1);
        let o2 = Orientation::triplet(self.0, other.1, self.1);
        let o3 = Orientation::triplet(other.0, self.0, other.1);
        let o4 = Orientation::triplet(other.0, self.1, other.1);

        if o1 != o2 && o3 != o4 {
            return true;
        }

        if o1 == Orientation::Collinear && on_segment(self.0, other.0, self.1) {
            return true;
        }
        if o2 == Orientation::Collinear && on_segment(self.0, other.1, self.1) {
            return true;
        }
        if o3 == Orientation::Collinear && on_segment(other.0, self.0, other.1) {
            return true;
        }
        if o4 == Orientation::Collinear && on_segment(other.0, self.1, other.1) {
            return true;
        }

        return false;
    }
}
