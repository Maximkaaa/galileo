#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Datum {
    semimajor: f64,
    inv_flattening: f64,
}

impl Datum {
    pub const WGS84: Self = Datum {
        semimajor: 6_378_137.0,
        inv_flattening: 298.257223563,
    };

    pub fn semimajor(&self) -> f64 {
        self.semimajor
    }

    pub fn inv_flattening(&self) -> f64 {
        self.inv_flattening
    }
}

impl Default for Datum {
    fn default() -> Self {
        Self::WGS84
    }
}
