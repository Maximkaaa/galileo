/// Projections convert points between different coordinate sysytems.
pub trait Projection {
    /// Point type that will be used as input for projecting.
    type InPoint;
    /// Resulting point type.
    type OutPoint;

    /// Convert point.
    fn project(&self, input: &Self::InPoint) -> Option<Self::OutPoint>;
    /// Convert point backwards.
    fn unproject(&self, input: &Self::OutPoint) -> Option<Self::InPoint>;

    /// Return inverse projection, e.g. a projection for which `project` does `unproject` and `unproject` does `project`.
    fn inverse(self: Box<Self>) -> InvertedProjection<Self::InPoint, Self::OutPoint>
    where
        Self: Sized + 'static,
    {
        InvertedProjection::new(self)
    }
}

/// Projection that does exactly opposite to what the base projection does.
pub struct InvertedProjection<IN, OUT> {
    inner: Box<dyn Projection<InPoint = IN, OutPoint = OUT>>,
}

impl<IN, OUT> InvertedProjection<IN, OUT> {
    /// Create a new instance with the base projection.
    pub fn new(inner: Box<dyn Projection<InPoint = IN, OutPoint = OUT>>) -> Self {
        Self { inner }
    }
}

impl<IN, OUT> Projection for InvertedProjection<IN, OUT> {
    type InPoint = OUT;
    type OutPoint = IN;

    fn project(&self, input: &Self::InPoint) -> Option<Self::OutPoint> {
        self.inner.unproject(input)
    }

    fn unproject(&self, input: &Self::OutPoint) -> Option<Self::InPoint> {
        self.inner.project(input)
    }
}

/// Chain two projections together.
///
/// The chain projection does two consequent projections on the point, e.g. first project it with the first projection,
/// and then project with the second projection, and return the result. `unproject` does projection in the reverse
/// order.
pub struct ChainProjection<IN, MID, OUT> {
    first: Box<dyn Projection<InPoint = IN, OutPoint = MID>>,
    second: Box<dyn Projection<InPoint = MID, OutPoint = OUT>>,
}

impl<IN, MID, OUT> ChainProjection<IN, MID, OUT> {
    /// Create a new instance.
    pub fn new(
        first: Box<dyn Projection<InPoint = IN, OutPoint = MID>>,
        second: Box<dyn Projection<InPoint = MID, OutPoint = OUT>>,
    ) -> Self {
        Self { first, second }
    }
}

impl<IN, MID, OUT> Projection for ChainProjection<IN, MID, OUT> {
    type InPoint = IN;
    type OutPoint = OUT;

    fn project(&self, input: &Self::InPoint) -> Option<Self::OutPoint> {
        self.second.project(&self.first.project(input)?)
    }

    fn unproject(&self, input: &Self::OutPoint) -> Option<Self::InPoint> {
        self.first.unproject(&self.second.unproject(input)?)
    }
}
