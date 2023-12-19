use crate::geo::datum::Datum;
use crate::geo::traits::point::{GeoPoint, NewGeoPoint};
use crate::geo::traits::projection::Projection;
use crate::{CartesianPoint2d, NewCartesianPoint2d};
use num_traits::Float;
use std::marker::PhantomData;

#[derive(Debug, Copy, Clone)]
pub struct WebMercator<In, Out> {
    datum: Datum,
    phantom_in: PhantomData<In>,
    phantom_out: PhantomData<Out>,
}

impl<In, Out> WebMercator<In, Out> {
    pub fn new(datum: Datum) -> Self {
        Self {
            datum,
            phantom_in: Default::default(),
            phantom_out: Default::default(),
        }
    }
}

impl<In, Out> Default for WebMercator<In, Out> {
    fn default() -> Self {
        Self {
            datum: Datum::WGS84,
            phantom_in: Default::default(),
            phantom_out: Default::default(),
        }
    }
}

impl<In: NewGeoPoint<f64>, Out: NewCartesianPoint2d<f64>> Projection for WebMercator<In, Out> {
    type InPoint = In;
    type OutPoint = Out;

    fn project(&self, input: &Self::InPoint) -> Option<Self::OutPoint> {
        let x = self.datum.semimajor() * input.lon_rad();
        let y = self.datum.semimajor()
            * (std::f64::consts::FRAC_PI_4 + input.lat_rad() / 2.0)
                .tan()
                .ln();

        if x.is_finite() && y.is_finite() {
            Some(Self::OutPoint::new(x, y))
        } else {
            None
        }
    }

    fn unproject(&self, input: &Self::OutPoint) -> Option<Self::InPoint> {
        let lat = std::f64::consts::FRAC_PI_2
            - 2.0
                * (-(*input).y() / self.datum.semimajor())
                    .powf(std::f64::consts::E)
                    .atan();
        let lon = input.x() / self.datum.semimajor();

        Some(Self::InPoint::latlon(lat, lon))
    }
}
