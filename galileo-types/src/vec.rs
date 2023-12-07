use std::ops::Mul;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Vec2d<Num: num_traits::Num + Copy> {
    pub(crate) dx: Num,
    pub(crate) dy: Num,
}

impl<Num: num_traits::Num + Copy> Vec2d<Num> {
    pub fn length_sq(&self) -> Num {
        self.dx * self.dx + self.dy * self.dy
    }
}

impl<Num: num_traits::Num + Copy> Mul<Num> for Vec2d<Num> {
    type Output = Self;

    fn mul(self, rhs: Num) -> Self::Output {
        Self {
            dx: self.dx * rhs,
            dy: self.dy * rhs,
        }
    }
}
