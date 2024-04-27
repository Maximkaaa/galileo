use num_traits::{FromPrimitive, NumCast};
use serde::{Deserialize, Serialize};

/// Generic size type. Size is not guaranteed to be non-negative.
#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd, Hash, Deserialize, Serialize)]
pub struct Size<Num: num_traits::Num + PartialOrd + Copy + PartialEq = f64> {
    width: Num,
    height: Num,
}

impl<Num: num_traits::Num + FromPrimitive + PartialOrd + Copy + NumCast> Size<Num> {
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
        self.width / Num::from_f64(2.0).expect("const conversion failed")
    }

    /// Height.
    pub fn height(&self) -> Num {
        self.height
    }

    /// Half height.
    pub fn half_height(&self) -> Num {
        self.height / Num::from_f64(2.0).expect("const conversion failed")
    }

    /// Returns true if both width and height are exactly zero.
    pub fn is_zero(&self) -> bool {
        self.width.is_zero() || self.height.is_zero()
    }

    /// Casts the underlying width and height values into the requested type.
    pub fn cast<T: num_traits::Num + FromPrimitive + PartialOrd + Copy + NumCast>(
        &self,
    ) -> Size<T> {
        Size {
            width: NumCast::from(self.width).expect("invalid value"),
            height: NumCast::from(self.height).expect("invalid value"),
        }
    }
}
