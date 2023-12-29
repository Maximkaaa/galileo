use crate::cartesian::traits::cartesian_point::CartesianPoint2d;

#[derive(Debug, PartialEq, Eq)]
pub enum Orientation {
    Clockwise,
    Counterclockwise,
    Collinear,
}

impl Orientation {
    pub fn triplet<Num: num_traits::Num + PartialOrd>(
        p: &impl CartesianPoint2d<Num = Num>,
        q: &impl CartesianPoint2d<Num = Num>,
        r: &impl CartesianPoint2d<Num = Num>,
    ) -> Self {
        match (q.y() - p.y()) * (r.x() - q.x()) - (q.x() - p.x()) * (r.y() - q.y()) {
            v if v == Num::zero() => Self::Collinear,
            v if v > Num::zero() => Self::Clockwise,
            v if v < Num::zero() => Self::Counterclockwise,
            _ => panic!("Invalid coordinates"),
        }
    }
}
