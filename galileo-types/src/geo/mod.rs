//! Geometries in geographic coordinates (latitude and longitude) (see [`GeoPoint`]) and conversion between different geographic
//! coordinate systems (see [`Projection`]).

mod crs;
mod datum;
pub mod impls;
mod traits;

pub use crs::{Crs, ProjectionType};
pub use datum::Datum;
pub use traits::point::{GeoPoint, NewGeoPoint};
pub use traits::projection::{ChainProjection, InvertedProjection, Projection};
