use serde::{Deserialize, Serialize};

use super::GeoPoint;

/// Reference ellipsoid used to do calculations with geographic coordinates.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Datum {
    semimajor: f64,
    semiminor: f64,
    inv_flattening: f64,
}

impl Datum {
    /// WGS84 ellipsoid
    pub const WGS84: Self = Datum {
        semimajor: 6_378_137.0,
        semiminor: 6_356_752.314_245,
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

    /// Flattening
    pub fn flattening(&self) -> f64 {
        1.0 / self.inv_flattening
    }

    /// Semiminor axis.
    pub fn semiminor(&self) -> f64 {
        self.semiminor
    }

    pub fn distance<A, B>(&self, a: &A, b: &B) -> f64
    where
        A: GeoPoint,
        B: GeoPoint<Num = <A as GeoPoint>::Num>,
    {
        super::traits::point::GeoPointExt::distance_accurate(a, b, self)
    }
}

impl Default for Datum {
    fn default() -> Self {
        Self::WGS84
    }
}
