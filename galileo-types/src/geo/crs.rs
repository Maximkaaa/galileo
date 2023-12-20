use crate::geo::datum::Datum;
use crate::geo::impls::projection::web_mercator::WebMercator;
use crate::geo::traits::point::NewGeoPoint;
use crate::geo::traits::projection::Projection;
use crate::NewCartesianPoint2d;

#[derive(Debug, Clone, PartialEq)]
pub struct Crs {
    datum: Datum,
    projection_type: ProjectionType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProjectionType {
    Unknown,
    None,
    WebMercator,
    Other(String),
}

impl Crs {
    pub const EPSG3857: Crs = Crs {
        datum: Datum::WGS84,
        projection_type: ProjectionType::WebMercator,
    };

    pub fn get_projection<In, Out>(
        &self,
    ) -> Option<Box<dyn Projection<InPoint = In, OutPoint = Out>>>
    where
        In: NewGeoPoint + 'static,
        Out: NewCartesianPoint2d + 'static,
    {
        match self.projection_type {
            ProjectionType::WebMercator => Some(Box::new(WebMercator::new(self.datum))),
            _ => None,
        }
    }
}
