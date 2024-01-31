use crate::layer::feature_layer::feature::Feature;

impl Feature for geojson::Feature {
    type Geom = geojson::Geometry;

    fn geometry(&self) -> &Self::Geom {
        let res = self.geometry.as_ref().unwrap();
        &res
    }
}
