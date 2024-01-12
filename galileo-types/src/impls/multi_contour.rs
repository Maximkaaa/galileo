use crate::cartesian::impls::contour::Contour;

pub struct MultiContour<P>(Vec<Contour<P>>);

impl<P> crate::multi_contour::MultiContour for MultiContour<P> {
    type Contour = Contour<P>;

    fn contours(&self) -> impl Iterator<Item = &Self::Contour> {
        self.0.iter()
    }
}

impl<P> From<Vec<Contour<P>>> for MultiContour<P> {
    fn from(value: Vec<Contour<P>>) -> Self {
        Self(value)
    }
}
