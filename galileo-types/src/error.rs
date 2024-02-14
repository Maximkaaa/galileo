//! Error type used by the crate.

use thiserror::Error;

/// Error enum.
#[derive(Debug, Error)]
pub enum GalileoTypesError {
    /// Geometry conversion error.
    #[error("invalid input geometry: {0}")]
    Conversion(String),
}
