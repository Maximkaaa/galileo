use crate::cartesian::impls::contour::ClosedContour;
use crate::cartesian::traits::cartesian_point::CartesianPoint2d;
use nalgebra::{Point2, Scalar};
use num_traits::{FromPrimitive, Num};
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rect<N = f64> {
    pub x_min: N,
    pub y_min: N,
    pub x_max: N,
    pub y_max: N,
}

impl<N: Num + Copy + PartialOrd + Scalar + FromPrimitive> Rect<N> {
    pub fn new(x_min: N, y_min: N, x_max: N, y_max: N) -> Self {
        Self {
            x_min,
            y_min,
            x_max,
            y_max,
        }
    }

    pub fn x_min(&self) -> N {
        self.x_min
    }

    pub fn x_max(&self) -> N {
        self.x_max
    }

    pub fn y_min(&self) -> N {
        self.y_min
    }

    pub fn y_max(&self) -> N {
        self.y_max
    }

    pub fn width(&self) -> N {
        self.x_max - self.x_min
    }

    pub fn height(&self) -> N {
        self.y_max - self.y_min
    }

    pub fn into_contour(self) -> ClosedContour<Point2<N>> {
        ClosedContour::new(Vec::from(self.into_quadrangle()))
    }

    pub fn shrink(&self, amount: N) -> Self {
        Self {
            x_min: self.x_min + amount,
            x_max: self.x_max - amount,
            y_min: self.y_min + amount,
            y_max: self.y_max - amount,
        }
    }

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

    pub fn magnify(&self, factor: N) -> Self {
        let two = N::from_f64(2.0).unwrap();
        let cx = (self.x_min + self.x_max) / two;
        let cy = (self.y_min + self.y_max) / two;
        let half_width = self.width() / two * factor;
        let half_height = self.height() / two * factor;
        Self {
            x_min: cx - half_width,
            x_max: cx + half_width,
            y_min: cy - half_height,
            y_max: cy + half_height,
        }
    }

    pub fn limit(&self, other: Self) -> Self {
        Self {
            x_min: if self.x_min > other.x_min {
                self.x_min
            } else {
                other.x_min
            },
            y_min: if self.y_min > other.y_min {
                self.y_min
            } else {
                other.y_min
            },
            x_max: if self.x_max < other.x_max {
                self.x_max
            } else {
                other.x_max
            },
            y_max: if self.y_max < other.y_max {
                self.y_max
            } else {
                other.y_max
            },
        }
    }

    pub fn center(&self) -> Point2<N> {
        Point2::new(
            (self.x_min + self.x_max) / N::from_f64(2.0).unwrap(),
            (self.y_min + self.y_max) / N::from_f64(2.0).unwrap(),
        )
    }

    pub fn into_quadrangle(self) -> [Point2<N>; 4] {
        [
            Point2::new(self.x_min, self.y_min),
            Point2::new(self.x_min, self.y_max),
            Point2::new(self.x_max, self.y_max),
            Point2::new(self.x_max, self.y_min),
        ]
    }
}

impl<N: Num + Copy + PartialOrd + Scalar + FromPrimitive> FromIterator<Rect<N>> for Rect<N> {
    fn from_iter<T: IntoIterator<Item = Rect<N>>>(iter: T) -> Self {
        let mut iter = iter.into_iter();
        let mut curr = iter.next().unwrap();
        for rect in iter {
            curr = curr.merge(rect);
        }

        curr
    }
}
