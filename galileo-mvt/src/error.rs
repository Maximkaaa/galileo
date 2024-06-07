use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum GalileoMvtError {
    #[error("proto error: {0}")]
    Proto(String),

    #[error("{0}")]
    Generic(String),
}