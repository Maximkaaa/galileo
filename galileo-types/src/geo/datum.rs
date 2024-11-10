use serde::{Deserialize, Serialize};

/// Reference ellipsoid used to do calculations with geographic coordinates.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Datum {
    semimajor: f64,
    inv_flattening: f64,
}

impl Datum {
    /// WGS84 ellipsoid
    pub const WGS84: Self = Datum {
        semimajor: 6_378_137.0,
        inv_flattening: 298.257223563,
    };

    /// Semimajor axis.
    pub fn semimajor(&self) -> f64 {
        self.semimajor
    }

    /// Inverse flattening.
    pub fn inv_flattening(&self) -> f64 {
        self.inv_flattening
    }
}

impl Default for Datum {
    fn default() -> Self {
        Self::WGS84
    }
}
