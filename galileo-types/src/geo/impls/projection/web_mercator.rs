use crate::cartesian::NewCartesianPoint2d;
use crate::geo::datum::Datum;
use crate::geo::traits::point::NewGeoPoint;
use crate::geo::traits::projection::Projection;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// Web Mercator projection.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct WebMercator<In, Out> {
    datum: Datum,
    phantom_in: PhantomData<In>,
    phantom_out: PhantomData<Out>,
}

impl<In, Out> WebMercator<In, Out> {
    /// Creates a new projection with the given `datum`.
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
            - 2.0 * (-(*input).y() / self.datum.semimajor()).exp().atan();
        let lon = input.x() / self.datum.semimajor();

        if !lat.is_finite() || !lon.is_finite() {
            return None;
        }

        Some(Self::InPoint::latlon(lat.to_degrees(), lon.to_degrees()))
    }
}
