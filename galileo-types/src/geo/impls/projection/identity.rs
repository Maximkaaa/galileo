use crate::cartesian::traits::cartesian_point::{NewCartesianPoint2d, NewCartesianPoint3d};
use crate::geo::traits::point::NewGeoPoint;
use crate::geo::traits::projection::Projection;
use crate::geometry_type::{CartesianSpace2d, CartesianSpace3d, GeoSpace2d};
use std::marker::PhantomData;

#[derive(Default)]
pub struct IdentityProjection<IN, OUT, Space> {
    phantom_in: PhantomData<IN>,
    phantom_out: PhantomData<OUT>,
    phantom_space: PhantomData<Space>,
}

impl<IN, OUT, Space> IdentityProjection<IN, OUT, Space> {
    pub fn new() -> Self {
        Self {
            phantom_in: Default::default(),
            phantom_out: Default::default(),
            phantom_space: Default::default(),
        }
    }
}

impl<IN: NewCartesianPoint2d, OUT: NewCartesianPoint2d> Projection
    for IdentityProjection<IN, OUT, CartesianSpace2d>
{
    type InPoint = IN;
    type OutPoint = OUT;

    fn project(&self, input: &Self::InPoint) -> Option<Self::OutPoint> {
        Some(OUT::new(input.x(), input.y()))
    }

    fn unproject(&self, input: &Self::OutPoint) -> Option<Self::InPoint> {
        Some(IN::new(input.x(), input.y()))
    }
}

impl<IN: NewCartesianPoint3d, OUT: NewCartesianPoint3d> Projection
    for IdentityProjection<IN, OUT, CartesianSpace3d>
{
    type InPoint = IN;
    type OutPoint = OUT;

    fn project(&self, input: &Self::InPoint) -> Option<Self::OutPoint> {
        Some(OUT::new(input.x(), input.y(), input.z()))
    }

    fn unproject(&self, input: &Self::OutPoint) -> Option<Self::InPoint> {
        Some(IN::new(input.x(), input.y(), input.z()))
    }
}

impl<IN: NewGeoPoint, OUT: NewGeoPoint> Projection for IdentityProjection<IN, OUT, GeoSpace2d> {
    type InPoint = IN;
    type OutPoint = OUT;

    fn project(&self, input: &Self::InPoint) -> Option<Self::OutPoint> {
        Some(OUT::latlon(input.lat(), input.lon()))
    }

    fn unproject(&self, input: &Self::OutPoint) -> Option<Self::InPoint> {
        Some(IN::latlon(input.lat(), input.lon()))
    }
}
