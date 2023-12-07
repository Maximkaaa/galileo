use prost::DecodeError;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum GalileoMvtError {
    #[error("proto error: {0}")]
    Proto(String),

    #[error("{0}")]
    Generic(String),
}

impl From<DecodeError> for GalileoMvtError {
    fn from(value: DecodeError) -> Self {
        Self::Proto(value.to_string())
    }
}
