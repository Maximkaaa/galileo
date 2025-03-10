use nalgebra::Scalar;
use num_traits::{FromPrimitive, NumCast, ToPrimitive};
use serde::{Deserialize, Serialize};

use super::{Rect, Vector2};

/// Generic size type. Size is not guaranteed to be non-negative.
#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd, Hash, Deserialize, Serialize)]
pub struct Size<Num = f64> {
    width: Num,
    height: Num,
}

impl<Num: num_traits::Num + Copy> Size<Num> {
    /// Creates a new instance.
    pub fn new(width: Num, height: Num) -> Self {
        Self { width, height }
    }

    /// Width.
    pub fn width(&self) -> Num {
        self.width
    }

    /// Half width.
    pub fn half_width(&self) -> Num {
        self.width / (Num::one() + Num::one())
    }

    /// Height.
    pub fn height(&self) -> Num {
        self.height
    }

    /// Half height.
    pub fn half_height(&self) -> Num {
        self.height / (Num::one() + Num::one())
    }

    /// Returns true if both width and height are exactly zero.
    pub fn is_zero(&self) -> bool {
        self.width.is_zero() || self.height.is_zero()
    }

    /// Casts the underlying width and height values into the requested type.
    pub fn cast<T>(&self) -> Size<T>
    where
        Num: ToPrimitive,
        T: NumCast,
    {
        Size {
            width: NumCast::from(self.width).expect("invalid value"),
            height: NumCast::from(self.height).expect("invalid value"),
        }
    }

    /// Converts size object to a rectangle with left-top point `(0, 0)`.
    pub fn to_rect(&self) -> Rect<Num>
    where
        Num: Scalar + FromPrimitive + PartialOrd,
    {
        Rect::new(Num::zero(), Num::zero(), self.width, self.height)
    }
}

impl<Num> std::ops::Mul<Vector2<Num>> for Size<Num>
where
    Num: std::ops::Mul<Num, Output = Num> + Copy,
{
    type Output = Size<Num>;

    fn mul(self, rhs: Vector2<Num>) -> Self::Output {
        Self {
            width: self.width * rhs.dx(),
            height: self.height * rhs.dy(),
        }
    }
}

impl<Num> std::ops::Mul<Num> for Size<Num>
where
    Num: std::ops::Mul<Num, Output = Num> + Copy,
{
    type Output = Size<Num>;

    fn mul(self, rhs: Num) -> Self::Output {
        Self {
            width: self.width * rhs,
            height: self.height * rhs,
        }
    }
}
