use galileo_types::geo::traits::point::GeoPoint;

pub trait Feature {
    type Geom;
    fn geometry(&self) -> &Self::Geom;
}

impl<T: GeoPoint> Feature for T {
    type Geom = T;

    fn geometry(&self) -> &Self::Geom {
        &self
    }
}