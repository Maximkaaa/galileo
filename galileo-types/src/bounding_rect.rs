use crate::CartesianPoint2d;
use num_traits::Num;
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundingRect<N: Num + Copy + PartialOrd = f64> {
    pub x_min: N,
    pub y_min: N,
    pub x_max: N,
    pub y_max: N,
}

impl<N: Num + Copy + PartialOrd> BoundingRect<N> {
    pub fn merge(&self, other: Self) -> Self {
        Self {
            x_min: if self.x_min < other.x_min {
                self.x_min
            } else {
                other.x_min
            },
            y_min: if self.y_min < other.y_min {
                self.y_min
            } else {
                other.y_min
            },
            x_max: if self.x_max > other.x_max {
                self.x_max
            } else {
                other.x_max
            },
            y_max: if self.y_max > other.y_max {
                self.y_max
            } else {
                other.y_max
            },
        }
    }

    pub fn from_point(p: &impl CartesianPoint2d<Num = N>) -> Self {
        Self {
            x_min: p.x(),
            x_max: p.x(),
            y_min: p.y(),
            y_max: p.y(),
        }
    }

    pub fn from_points<'a, P: CartesianPoint2d<Num = N> + 'a>(
        mut points: impl Iterator<Item = &'a P>,
    ) -> Option<Self> {
        let first = points.next()?;
        let mut x_min = first.x();
        let mut y_min = first.y();
        let mut x_max = first.x();
        let mut y_max = first.y();

        for p in points {
            if x_min > p.x() {
                x_min = p.x();
            }
            if y_min > p.y() {
                y_min = p.y();
            }
            if x_max < p.x() {
                x_max = p.x();
            }
            if y_max < p.y() {
                y_max = p.y();
            }
        }

        Some(Self {
            x_min,
            y_min,
            x_max,
            y_max,
        })
    }

    pub fn contains(&self, point: &impl CartesianPoint2d<Num = N>) -> bool {
        self.x_min <= point.x()
            && self.x_max >= point.x()
            && self.y_min <= point.y()
            && self.y_max >= point.y()
    }
}

impl<N: Num + Copy + PartialOrd> FromIterator<BoundingRect<N>> for BoundingRect<N> {
    fn from_iter<T: IntoIterator<Item = BoundingRect<N>>>(iter: T) -> Self {
        let mut iter = iter.into_iter();
        let mut curr = iter.next().unwrap();
        for rect in iter {
            curr = curr.merge(rect);
        }

        curr
    }
}
