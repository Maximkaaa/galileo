use num_traits::{FromPrimitive, NumCast};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Size<Num: num_traits::Num + PartialOrd + Copy + PartialEq = f64> {
    width: Num,
    height: Num,
}

impl<Num: num_traits::Num + FromPrimitive + PartialOrd + Copy + NumCast> Size<Num> {
    pub fn new(width: Num, height: Num) -> Self {
        Self { width, height }
    }

    pub fn width(&self) -> Num {
        self.width
    }

    pub fn half_width(&self) -> Num {
        self.width / Num::from_f64(2.0).expect("const conversion failed")
    }

    pub fn height(&self) -> Num {
        self.height
    }

    pub fn half_height(&self) -> Num {
        self.height / Num::from_f64(2.0).expect("const conversion failed")
    }

    pub fn is_zero(&self) -> bool {
        self.width.is_zero() || self.height.is_zero()
    }

    pub fn cast<T: num_traits::Num + FromPrimitive + PartialOrd + Copy + NumCast>(
        &self,
    ) -> Size<T> {
        Size {
            width: NumCast::from(self.width).expect("invalid value"),
            height: NumCast::from(self.height).expect("invalid value"),
        }
    }
}
