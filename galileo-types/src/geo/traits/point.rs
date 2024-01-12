use crate::geo::datum::Datum;
use crate::geo::traits::projection::Projection;
use crate::geometry::{Geom, GeometrySpecialization};
use crate::geometry_type::{GeoSpace2d, GeometryType, PointGeometryType};
use crate::point::{GeoPointType, Point, PointHelper};
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

    fn distance(
        &self,
        _other: &impl GeoPoint<Num = Self::Num>,
        _datum: &Datum,
    ) -> Option<Self::Num> {
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

impl<P> GeometrySpecialization<PointGeometryType, GeoSpace2d> for P
where
    P: GeoPoint + GeometryType<Type = PointGeometryType, Space = GeoSpace2d>,
{
    type Point = P;

    fn project<Proj>(&self, projection: &Proj) -> Option<Geom<Proj::OutPoint>>
    where
        Proj: Projection<InPoint = Self::Point> + ?Sized,
    {
        Some(Geom::Point(projection.project(self)?))
    }
}
