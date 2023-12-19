pub trait Projection {
    type InPoint;
    type OutPoint;

    fn project(&self, input: &Self::InPoint) -> Option<Self::OutPoint>;
    fn unproject(&self, input: &Self::OutPoint) -> Option<Self::InPoint>;
}
