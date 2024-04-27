use crate::cartesian::CartesianPoint2d;
use serde::{Deserialize, Serialize};

/// Orientation of a triplet of points.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Orientation {
    /// Clockwise
    Clockwise,
    /// Counterclockwise
    Counterclockwise,
    /// Collinear
    Collinear,
}

impl Orientation {
    /// Determines orientation of a triplet of points.
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
