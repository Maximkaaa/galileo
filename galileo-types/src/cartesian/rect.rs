use std::ops::Deref;

use nalgebra::Scalar;
use num_traits::{FromPrimitive, Num};
use serde::{Deserialize, Serialize};

use super::{Point2, Vector2};
use crate::cartesian::CartesianPoint2d;
use crate::impls::ClosedContour;

/// Rectangle in 2d cartesian coordinate space.
#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd, Hash, Deserialize, Serialize)]
pub struct Rect<N = f64> {
    x_min: N,
    y_min: N,
    x_max: N,
    y_max: N,
}

impl<N> Rect<N>
where
    N: Num + Copy + PartialOrd + Scalar + FromPrimitive,
{
    /// Creates a new rectangle.
    pub fn new(x_min: N, y_min: N, x_max: N, y_max: N) -> Self {
        let (x_min, x_max) = if x_min > x_max {
            (x_max, x_min)
        } else {
            (x_min, x_max)
        };
        let (y_min, y_max) = if y_min > y_max {
            (y_max, y_min)
        } else {
            (y_min, y_max)
        };

        Self {
            x_min,
            y_min,
            x_max,
            y_max,
        }
    }

    /// X min.
    pub fn x_min(&self) -> N {
        self.x_min
    }

    /// X max.
    pub fn x_max(&self) -> N {
        self.x_max
    }

    /// Y min.
    pub fn y_min(&self) -> N {
        self.y_min
    }

    /// Y max.
    pub fn y_max(&self) -> N {
        self.y_max
    }

    /// Width of the rectangle. Guaranteed to be non-negative.
    pub fn width(&self) -> N {
        self.x_max - self.x_min
    }

    /// Height of the rectangle. Guaranteed to be non-negative.
    pub fn height(&self) -> N {
        self.y_max - self.y_min
    }

    /// Half of the width of the rectangle. Guaranteed to be non-negative.
    pub fn half_width(&self) -> N {
        self.width() / N::from_f64(2.0).expect("const conversion")
    }

    /// Half of the height of the rectangle. Guaranteed to be non-negative.
    pub fn half_height(&self) -> N {
        self.height() / N::from_f64(2.0).expect("const conversion")
    }

    /// Converts the rectangle into a closed contour of points.
    pub fn into_contour(self) -> ClosedContour<Point2<N>> {
        ClosedContour::new(Vec::from(self.into_quadrangle()))
    }

    /// Moves the boundaries of the rectangle by `amount` inside (outside, if the `amount` is negative). If the
    /// width or height of the resulting rectangle are negative, they are set to 0.
    pub fn shrink(&self, amount: N) -> Self {
        let amount_x = if amount <= self.half_width() {
            amount
        } else {
            self.half_width()
        };
        let amount_y = if amount <= self.half_height() {
            amount
        } else {
            self.half_height()
        };

        Self {
            x_min: self.x_min + amount_x,
            x_max: self.x_max - amount_x,
            y_min: self.y_min + amount_y,
            y_max: self.y_max - amount_y,
        }
    }

    /// Adds the given amount to the coordinates of the rectangle.
    pub fn shift(&self, dx: N, dy: N) -> Self {
        Self {
            x_min: self.x_min + dx,
            x_max: self.x_max + dx,
            y_min: self.y_min + dy,
            y_max: self.y_max + dy,
        }
    }

    /// Creates a new rectangle with the boundaries of this and other one.
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

    /// Creates a zero-area rectangle from the point.
    pub fn from_point(p: &impl CartesianPoint2d<Num = N>) -> Self {
        Self {
            x_min: p.x(),
            x_max: p.x(),
            y_min: p.y(),
            y_max: p.y(),
        }
    }

    /// Returns a minimum rectangle that contains all the points in the iterator.
    ///
    /// Returns `None` if the iterator is empty.
    pub fn from_points<'a, P: CartesianPoint2d<Num = N> + 'a, T: Deref<Target = P> + 'a>(
        points: impl IntoIterator<Item = T>,
    ) -> Option<Self> {
        let mut iterator = points.into_iter();
        let first = iterator.next()?;
        let mut x_min = first.x();
        let mut y_min = first.y();
        let mut x_max = first.x();
        let mut y_max = first.y();

        for p in iterator {
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

    /// Returns `true` if the point is inside (or on a side) of the rectagle.
    pub fn contains(&self, point: &impl CartesianPoint2d<Num = N>) -> bool {
        self.x_min <= point.x()
            && self.x_max >= point.x()
            && self.y_min <= point.y()
            && self.y_max >= point.y()
    }

    /// Changes the width and height of the rectangle by the factor of `factor`, keeping the center of the rectangle
    /// at the same place.
    pub fn magnify(&self, factor: N) -> Self {
        let two = N::from_f64(2.0).expect("const conversion failed");
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

    /// Returns a new rectangle, boundaries of which are inside of boundaries of this and the `other` rectangles.
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

    /// Returns the center point of the rectangle.
    pub fn center(&self) -> Point2<N> {
        Point2::new(
            (self.x_min + self.x_max) / N::from_f64(2.0).expect("const conversion failed"),
            (self.y_min + self.y_max) / N::from_f64(2.0).expect("const conversion failed"),
        )
    }

    /// Returns a set of 4 points - corners of the rectangle.
    ///
    /// The order of points is:
    /// 1. Left bottom
    /// 2. Left top
    /// 3. Right top
    /// 4. Right bottom
    pub fn into_quadrangle(self) -> [Point2<N>; 4] {
        [
            Point2::new(self.x_min, self.y_min),
            Point2::new(self.x_min, self.y_max),
            Point2::new(self.x_max, self.y_max),
            Point2::new(self.x_max, self.y_min),
        ]
    }

    /// Returns true if two rectangle have at least one common point.
    pub fn intersects(&self, other: Rect<N>) -> bool {
        self.x_max >= other.x_min
            && self.x_min <= other.x_max
            && self.y_max >= other.y_min
            && self.y_min <= other.y_max
    }
}

impl<N: Num + Copy + PartialOrd + Scalar + FromPrimitive> FromIterator<Rect<N>>
    for Option<Rect<N>>
{
    fn from_iter<T: IntoIterator<Item = Rect<N>>>(iter: T) -> Self {
        let mut iter = iter.into_iter();
        let mut prev = iter.next()?;
        for next in iter {
            prev = prev.merge(next);
        }

        Some(prev)
    }
}

impl<N> std::ops::Add<Vector2<N>> for Rect<N>
where
    N: Num + Copy + PartialOrd + Scalar + FromPrimitive,
{
    type Output = Rect<N>;

    fn add(self, rhs: Vector2<N>) -> Self::Output {
        Self {
            x_min: self.x_min + rhs.dx(),
            y_min: self.y_min + rhs.dy(),
            x_max: self.x_max + rhs.dx(),
            y_max: self.y_max + rhs.dy(),
        }
    }
}

impl<N> std::ops::Sub<Vector2<N>> for Rect<N>
where
    N: Num + Copy + PartialOrd + Scalar + FromPrimitive,
{
    type Output = Rect<N>;

    fn sub(self, rhs: Vector2<N>) -> Self::Output {
        Self {
            x_min: self.x_min - rhs.dx(),
            y_min: self.y_min - rhs.dy(),
            x_max: self.x_max - rhs.dx(),
            y_max: self.y_max - rhs.dy(),
        }
    }
}
