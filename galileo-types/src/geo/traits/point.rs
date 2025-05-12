use nalgebra::Vector2;
use num_traits::{Float, ToPrimitive as _};

use crate::geo::traits::projection::Projection;
use crate::geo::Datum;
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

pub trait GeoPointExt: GeoPoint {
    fn difference_normalized<G: GeoPoint<Num = Self::Num>>(&self, other: &G) -> Vector2<Self::Num> {
        Vector2::new(
            self.lat() - other.lat(),
            self.lon() - other.lon() * other.lat().to_radians().cos(),
        )
    }
    fn distance<G: GeoPoint<Num = Self::Num>>(&self, other: &G) -> f64 {
        const EARTH_RADIUS: f64 = 6371008.8; // WGS84 mean radius in meters

        let lat1 = self.lat_rad().to_f64().expect("convertible to f64");
        let lon1 = self.lon_rad().to_f64().expect("Convertible to f64");
        let lat2 = other.lat_rad().to_f64().expect("Convertible to f64");
        let lon2 = other.lon_rad().to_f64().expect("Convertible to f64");

        let dlat = lat2 - lat1;
        let dlon = lon2 - lon1;

        let a = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

        c * EARTH_RADIUS
    }
    fn distance_accurate<G: GeoPoint<Num = Self::Num>>(&self, other: &G, datum: &Datum) -> f64 {
        let smaj = datum.semimajor();
        let smin = datum.semiminor();
        let flattening = 1. / datum.flattening();
        let lat1 = self.lat_rad().to_f64().expect("convertible to f64");
        let lon1 = self.lon_rad().to_f64().expect("Convertible to f64");
        let lat2 = other.lat_rad().to_f64().expect("Convertible to f64");
        let lon2 = other.lon_rad().to_f64().expect("Convertible to f64");

        let l = lon2 - lon1;
        fn vincenty(
            flattening: f64,
            lat1: f64,
            lat2: f64,
            l: f64,
        ) -> Option<(f64, f64, f64, f64, f64)> {
            let (sin_u1, cos_u1) = ((1.0 - flattening) * lat1.tan()).atan().sin_cos();
            let (sin_u2, cos_u2) = ((1.0 - flattening) * lat2.tan()).atan().sin_cos();
            let mut lambda_prev = l;
            for _ in 0..100 {
                let (sin_lambda, cos_lambda) = lambda_prev.sin_cos();
                let sin_sigma = ((cos_u2 * sin_lambda).powi(2)
                    + (cos_u1 * sin_u2 - sin_u1 * cos_u2 * cos_lambda).powi(2))
                .sqrt();

                if sin_sigma == 0.0 {
                    return None; // Coincident points
                }

                let cos_sigma = sin_u1 * sin_u2 + cos_u1 * cos_u2 * cos_lambda;
                let sigma = sin_sigma.atan2(cos_sigma);
                let sin_alpha = cos_u1 * cos_u2 * sin_lambda / sin_sigma;
                let cos_sq_alpha = 1.0 - sin_alpha.powi(2);
                let cos2_sigma_m = if cos_sq_alpha != 0.0 {
                    cos_sigma - 2.0 * sin_u1 * sin_u2 / cos_sq_alpha
                } else {
                    0.0 // Equatorial line
                };

                let c = flattening / 16.0
                    * cos_sq_alpha
                    * (4.0 + flattening * (4.0 - 3.0 * cos_sq_alpha));
                let k_sigma = sigma
                    + c * sin_sigma
                        * (cos2_sigma_m + c * cos_sigma * (-1.0 + 2.0 * cos2_sigma_m.powi(2)));
                let lambda = l + (1.0 - c) * flattening * sin_alpha * k_sigma;

                if (lambda - lambda_prev).abs() <= 1e-12 {
                    return Some((cos_sq_alpha, sin_sigma, cos_sigma, cos2_sigma_m, sigma));
                }
                lambda_prev = lambda;
            }
            None
        }
        let Some((cos_sq_alpha, sin_sigma, cos_sigma, cos2_sigma_m, sigma)) =
            vincenty(flattening, lat1, lat2, l)
        else {
            return 0.0;
        };

        let u_sq = cos_sq_alpha * (smaj.powi(2) - smin.powi(2)) / smin.powi(2);
        let a = 1.0 + u_sq / 16384.0 * (4096.0 + u_sq * (-768.0 + u_sq * (320.0 - 175.0 * u_sq)));
        let b = u_sq / 1024.0 * (256.0 + u_sq * (-128.0 + u_sq * (74.0 - 47.0 * u_sq)));
        let c = b / 6.0
            * cos2_sigma_m
            * (-3.0 + 4.0 * sin_sigma.powi(2))
            * (-3.0 + 4.0 * cos2_sigma_m.powi(2));
        let d = cos_sigma * (-1.0 + 2.0 * cos2_sigma_m.powi(2)) - c;
        let e = cos2_sigma_m + b / 4.0 * d;
        let delta_sigma = b * sin_sigma * e;

        smin * a * (sigma - delta_sigma)
    }
}

impl<P: GeoPoint> GeoPointExt for P {}

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
