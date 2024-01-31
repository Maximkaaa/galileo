use thiserror::Error;

#[derive(Debug, Error)]
pub enum GalileoTypesError {
    #[error("invalid input geometry: {0}")]
    Conversion(String),
}
