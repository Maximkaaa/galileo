use crate::cartesian::traits::cartesian_point::NewCartesianPoint2d;
use crate::geo::traits::point::NewGeoPoint;
use crate::geo::traits::projection::Projection;
use geodesy::prelude::*;
use std::marker::PhantomData;

pub struct GeodesyProjection<In, Out> {
    context: Minimal,
    op: OpHandle,
    phantom_in: PhantomData<In>,
    phantom_out: PhantomData<Out>,
}

impl<In, Out> GeodesyProjection<In, Out> {
    pub fn new(definition: &str) -> Option<Self> {
        let mut context = Minimal::new();
        let op = context.op(definition).ok()?;
        Some(Self {
            context,
            op,
            phantom_in: Default::default(),
            phantom_out: Default::default(),
        })
    }
}

impl<In: NewGeoPoint<f64>, Out: NewCartesianPoint2d<f64>> Projection
    for GeodesyProjection<In, Out>
{
    type InPoint = In;
    type OutPoint = Out;

    fn project(&self, input: &Self::InPoint) -> Option<Self::OutPoint> {
        let mut data = [Coor2D::geo(input.lat(), input.lon())];
        self.context.apply(self.op, Fwd, &mut data).ok()?;

        if !data[0].0[0].is_finite() || !data[0].0[1].is_finite() {
            return None;
        }

        Some(Out::new(data[0].0[0], data[0].0[1]))
    }

    fn unproject(&self, input: &Self::OutPoint) -> Option<Self::InPoint> {
        let mut data = [Coor2D([input.x(), input.y()])];
        self.context.apply(self.op, Inv, &mut data).ok()?;

        Some(In::latlon(
            data[0].0[1].to_degrees(),
            data[0].0[0].to_degrees(),
        ))
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
    // use crate::cartesian::impls::point::Point2d;
    // use crate::geo::impls::point::GeoPoint2d;

    #[test]
    fn lambert_projection() {
        // todo: uncomment after geodesy crate is updated on crates.io
        // let pr = GeodesyProjection::new("laea lon_0=10 lat_0=52 x_0=4321000 y_0=3210000").unwrap();
        // let center = GeoPoint2d::latlon(52.0, 10.0);
        // let projected: Point2d = pr.project(&center).unwrap();
        // let unprojected = pr.unproject(&projected).unwrap();
        //
        // dbg!(center, projected, unprojected);
        // assert_eq!(center, unprojected);
    }
}
