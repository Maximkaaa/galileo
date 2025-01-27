use num_traits::Float;

use crate::geo::traits::projection::Projection;
use crate::geometry::{Geom, GeometrySpecialization};
use crate::geometry_type::{GeoSpace2d, GeometryType, PointGeometryType};

/// 2d point on the surface of a celestial body.
pub trait GeoPoint {
    /// Numeric type used to represent coordinates.
    type Num: Float;

    /// Latitude in degrees.
    fn lat(&self) -> Self::Num;
    /// Longitude in degrees.
    fn lon(&self) -> Self::Num;

    /// Latitude in radians.
    fn lat_rad(&self) -> Self::Num {
        self.lat().to_radians()
    }

    /// Longitude in radians.
    fn lon_rad(&self) -> Self::Num {
        self.lon().to_radians()
    }
}

/// Trait for points that can be constructed by only coordinates.
pub trait NewGeoPoint<N = f64>: GeoPoint<Num = N> + Sized {
    /// Create a point from *latitude* and *longitude*.
    fn latlon(lat: N, lon: N) -> Self;
    /// Create a point from *longitude* and *latitude*.
    fn lonlat(lon: N, lat: N) -> Self {
        Self::latlon(lat, lon)
    }
}

impl<P> GeometrySpecialization<PointGeometryType, GeoSpace2d> for P
where
    P: GeoPoint + GeometryType<Type = PointGeometryType, Space = GeoSpace2d>,
{
    type Point = P;

    fn project_spec<Proj>(&self, projection: &Proj) -> Option<Geom<Proj::OutPoint>>
    where
        Proj: Projection<InPoint = Self::Point> + ?Sized,
    {
        Some(Geom::Point(projection.project(self)?))
    }
}
