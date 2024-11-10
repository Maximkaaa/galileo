use crate::layer::feature_layer::feature::Feature;

impl Feature for geojson::Feature {
    type Geom = geojson::Geometry;

    fn geometry(&self) -> &Self::Geom {
        self.geometry
            .as_ref()
            .expect("GeoJSON Feature has no geometry")
    }
}
