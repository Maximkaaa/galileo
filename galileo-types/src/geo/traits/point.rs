use crate::geo::datum::Datum;
use crate::geometry::{GeoPointType, Point, PointHelper};
use num_traits::Float;

pub trait GeoPoint {
    type Num: Float;

    fn lat(&self) -> Self::Num;
    fn lon(&self) -> Self::Num;

    fn lat_rad(&self) -> Self::Num {
        self.lat().to_radians()
    }

    fn lon_rad(&self) -> Self::Num {
        self.lon().to_radians()
    }

    fn distance(&self, other: &impl GeoPoint<Num = Self::Num>, datum: &Datum) -> Option<Self::Num> {
        todo!()
    }
}

pub trait NewGeoPoint<N = f64>: GeoPoint<Num = N> + Sized {
    fn latlon(lat: N, lon: N) -> Self;
    fn lonlat(lon: N, lat: N) -> Self {
        Self::latlon(lat, lon)
    }
}

impl<T> PointHelper<GeoPointType> for T where T: GeoPoint + Point<Type = GeoPointType> {}
