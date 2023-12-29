use num_traits::real::Real;
use num_traits::FromPrimitive;

#[derive(Debug, Clone, Copy, Default)]
pub struct Size<Num: num_traits::Num + PartialOrd + Copy = f64> {
    width: Num,
    height: Num,
}

impl<Num: Real + FromPrimitive> Size<Num> {
    pub fn new(width: Num, height: Num) -> Self {
        Self {
            width: width.max(Num::zero()),
            height: height.max(Num::zero()),
        }
    }

    pub fn width(&self) -> Num {
        self.width
    }

    pub fn half_width(&self) -> Num {
        self.width / Num::from_f64(2.0).unwrap()
    }

    pub fn height(&self) -> Num {
        self.height
    }

    pub fn half_height(&self) -> Num {
        self.height / Num::from_f64(2.0).unwrap()
    }

    pub fn is_zero(&self) -> bool {
        self.width.is_zero() || self.height.is_zero()
    }
}
