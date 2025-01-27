use serde::{Deserialize, Serialize};

use crate::cartesian::NewCartesianPoint2d;
use crate::geo::datum::Datum;
use crate::geo::impls::projection::{GeodesyProjection, WebMercator};
use crate::geo::traits::point::NewGeoPoint;
use crate::geo::traits::projection::Projection;

/// Coordinate reference system.
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Crs {
    datum: Datum,
    projection_type: ProjectionType,
}

/// Method used for projecting coordinates.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ProjectionType {
    /// Some method.
    Unknown,
    /// No projection is used. The coordinates used by the CRS are *latitude* and *longitude*.
    None,
    /// Web Mercator projection.
    WebMercator,
    /// `proj` or `geodesy` definition of the projection.
    Other(String),
}

impl Crs {
    /// Standard Web Mercator coordinate system used by most web GIS applications.
    pub const EPSG3857: Crs = Crs {
        datum: Datum::WGS84,
        projection_type: ProjectionType::WebMercator,
    };

    /// Coordinate system in geographic coordinates with WGS84 datum.
    pub const WGS84: Crs = Crs {
        datum: Datum::WGS84,
        projection_type: ProjectionType::None,
    };

    /// Creates a new CRS.
    pub fn new(datum: Datum, projection_type: ProjectionType) -> Self {
        Self {
            datum,
            projection_type,
        }
    }

    /// Returns a projection that converts geographic coordinates into the coordinates of this CRS.
    ///
    /// Returns `None` if the CRS coordinates cannot be projected from geographic coordinates.
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
