use galileo::layer::feature::feature::Feature;
use galileo::primitives::Color;
use galileo_types::cartesian::impls::multipolygon::MultiPolygon;
use galileo_types::cartesian::impls::point::Point2d;
use galileo_types::cartesian::impls::polygon::Polygon;
use galileo_types::cartesian::rect::Rect;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Country {
    pub name: String,
    #[serde(deserialize_with = "des_geometry")]
    pub geometry: MultiPolygon<Point2d>,
    pub color: Color,
    pub bbox: Rect,
    pub is_selected: bool,
}

fn des_geometry<'de, D: Deserializer<'de>>(d: D) -> Result<MultiPolygon<Point2d>, D::Error> {
    Ok(Vec::<Polygon<Point2d>>::deserialize(d)?.into())
}

impl Country {
    pub fn is_selected(&self) -> bool {
        self.is_selected
    }
}

impl Feature for Country {
    type Geom = MultiPolygon<Point2d>;

    fn geometry(&self) -> &MultiPolygon<Point2d> {
        &self.geometry
    }
}

pub fn load_countries() -> Vec<Country> {
    bincode::deserialize(include_bytes!("countries.data")).unwrap()
}
