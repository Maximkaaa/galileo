use crate::geo::datum::Datum;
use crate::geo::impls::point::GeoPoint2d;
use crate::geo::impls::projection::web_mercator::WebMercator;
use crate::geo::traits::projection::Projection;
use crate::{CartesianPoint2d, NewCartesianPoint2d, Point2d};
use crate::geo::traits::point::{GeoPoint, NewGeoPoint};

#[derive(Debug, Clone)]
pub struct Crs {
    datum: Datum,
    projection_type: ProjectionType,
}

#[derive(Debug, Clone)]
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
