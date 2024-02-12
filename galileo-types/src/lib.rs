#![warn(clippy::unwrap_used)]

pub mod cartesian;
pub mod contour;
pub mod disambig;
pub mod error;
pub mod geo;
pub mod geometry;
pub mod geometry_type;
pub mod impls;
pub mod multi_contour;
pub mod multi_point;
pub mod multi_polygon;
pub mod point;
pub mod polygon;
pub mod segment;

#[cfg(feature = "geo-types")]
pub mod geo_types;

#[cfg(feature = "geojson")]
mod geojson;
