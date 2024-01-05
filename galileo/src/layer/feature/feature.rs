use galileo_types::geometry::Geometry;

pub trait Feature {
    type Geom;
    fn geometry(&self) -> &Self::Geom;
}

impl<T: Geometry> Feature for T {
    type Geom = T;

    fn geometry(&self) -> &Self::Geom {
        &self
    }
}
