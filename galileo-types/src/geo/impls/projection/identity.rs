use crate::geo::traits::projection::Projection;
use std::marker::PhantomData;

#[derive(Default)]
pub struct IdentityProjection<P> {
    phantom: PhantomData<P>,
}

impl<P> IdentityProjection<P> {
    pub fn new() -> Self {
        Self {
            phantom: Default::default(),
        }
    }
}

impl<P: Clone> Projection for IdentityProjection<P> {
    type InPoint = P;
    type OutPoint = P;

    fn project(&self, input: &Self::InPoint) -> Option<Self::OutPoint> {
        Some(input.clone())
    }

    fn unproject(&self, input: &Self::OutPoint) -> Option<Self::InPoint> {
        Some(input.clone())
    }
}
