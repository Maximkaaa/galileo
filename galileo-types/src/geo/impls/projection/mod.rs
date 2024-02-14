//! Implementations for some of the common projections.
mod dimensions;
mod identity;
mod web_mercator;

pub use dimensions::AddDimensionProjection;
pub use identity::IdentityProjection;
pub use web_mercator::WebMercator;

#[cfg(feature = "geodesy")]
mod geodesy;
#[cfg(feature = "geodesy")]
pub use geodesy::GeodesyProjection;
