use crate::cartesian::traits::cartesian_point::NewCartesianPoint2d;
use crate::geo::datum::Datum;
use crate::geo::impls::projection::geodesy::GeodesyProjection;
use crate::geo::impls::projection::web_mercator::WebMercator;
use crate::geo::traits::point::NewGeoPoint;
use crate::geo::traits::projection::Projection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Crs {
    datum: Datum,
    projection_type: ProjectionType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    pub const WGS84: Crs = Crs {
        datum: Datum::WGS84,
        projection_type: ProjectionType::None,
    };

    pub fn new(datum: Datum, projection_type: ProjectionType) -> Self {
        Self {
            datum,
            projection_type,
        }
    }

    pub fn get_projection<In, Out>(
        &self,
    ) -> Option<Box<dyn Projection<InPoint = In, OutPoint = Out>>>
    where
        In: NewGeoPoint + 'static,
        Out: NewCartesianPoint2d + 'static,
    {
        match &self.projection_type {
            ProjectionType::WebMercator => Some(Box::new(WebMercator::new(self.datum))),
            ProjectionType::Other(definition) => {
                Some(Box::new(GeodesyProjection::new(definition)?))
            }
            _ => None,
        }
    }
}
