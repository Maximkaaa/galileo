use crate::cartesian::{NewCartesianPoint2d, NewCartesianPoint3d};
use crate::geo::traits::projection::Projection;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// Projection that adds a default z-value to a 2d point. Reversed projecting drops the z-value.
#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd, Hash, Deserialize, Serialize)]
pub struct AddDimensionProjection<Num, In, Out> {
    z: Num,
    phantom_in: PhantomData<In>,
    phantom_out: PhantomData<Out>,
}

impl<Num, In, Out> AddDimensionProjection<Num, In, Out> {
    /// Creates a new instance, that will set the specified `z` value when projecting.
    pub fn new(z: Num) -> Self {
        Self {
            z,
            phantom_in: Default::default(),
            phantom_out: Default::default(),
        }
    }
}

impl<Num: Copy, In: NewCartesianPoint2d<Num>, Out: NewCartesianPoint3d<Num>> Projection
    for AddDimensionProjection<Num, In, Out>
{
    type InPoint = In;
    type OutPoint = Out;

    fn project(&self, input: &Self::InPoint) -> Option<Self::OutPoint> {
        Some(Out::new(input.x(), input.y(), self.z))
    }

    fn unproject(&self, input: &Self::OutPoint) -> Option<Self::InPoint> {
        Some(In::new(input.x(), input.y()))
    }
}
