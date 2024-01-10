pub trait Projection {
    type InPoint;
    type OutPoint;

    fn project(&self, input: &Self::InPoint) -> Option<Self::OutPoint>;
    fn unproject(&self, input: &Self::OutPoint) -> Option<Self::InPoint>;

    fn inverse(self: Box<Self>) -> InvertedProjection<Self::InPoint, Self::OutPoint>
    where
        Self: Sized + 'static,
    {
        InvertedProjection::new(self)
    }
}

pub struct InvertedProjection<IN, OUT> {
    inner: Box<dyn Projection<InPoint = IN, OutPoint = OUT>>,
}

impl<IN, OUT> InvertedProjection<IN, OUT> {
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

pub struct ChainProjection<IN, MID, OUT> {
    first: Box<dyn Projection<InPoint = IN, OutPoint = MID>>,
    second: Box<dyn Projection<InPoint = MID, OutPoint = OUT>>,
}

impl<IN, MID, OUT> ChainProjection<IN, MID, OUT> {
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
