use galileo::primitives::{Color, Point2d, Polygon};
use galileo_types::geometry::CartesianGeometry;
use galileo_types::rect::Rect;
use galileo_types::CartesianPoint2d;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Country {
    pub name: String,
    pub geometry: Vec<Polygon<Point2d>>,
    pub color: Color,
    pub bbox: Rect,
    pub is_selected: bool,
}

impl Country {
    pub fn is_selected(&self) -> bool {
        self.is_selected
    }
}

impl CartesianGeometry for Country {
    type Num = f64;

    fn bounding_rect(&self) -> Rect<Self::Num> {
        self.bbox
    }

    fn is_point_inside<P>(&self, point: &P, tolerance: Self::Num) -> bool
    where
        P: CartesianPoint2d<Num = Self::Num>,
    {
        self.bbox.contains(point)
            && self
                .geometry
                .iter()
                .any(|p| p.is_point_inside(point, tolerance))
    }
}

pub fn load_countries() -> Vec<Country> {
    bincode::deserialize(include_bytes!("countries.data")).unwrap()
}
