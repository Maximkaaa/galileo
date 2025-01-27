//! Error types used by the crate.

use std::io::Error;

use galileo_mvt::error::GalileoMvtError;
use thiserror::Error;

/// Galileo error type.
#[derive(Debug, Error, Clone)]
pub enum GalileoError {
    /// I/O error (network or file)
    #[error("failed to load data")]
    IO,
    /// Error decoding data.
    #[error("failed to decode data")]
    Decoding(#[from] GalileoMvtError),
    /// Error interacting with WASM runtime.
    #[error("wasm error: {0:?}")]
    Wasm(Option<String>),
    /// Item not found.
    #[error("item not found")]
    NotFound,
    /// Image decoding error.
    #[cfg(feature = "image")]
    #[error("image decode error")]
    ImageDecode,
    /// Generic error - details are inside.
    #[error("{0}")]
    Generic(String),
    /// Error reading/writing data to the FS.
    #[error("failed to read file")]
    FsIo,
}

#[cfg(not(target_arch = "wasm32"))]
impl From<reqwest::Error> for GalileoError {
    fn from(_value: reqwest::Error) -> Self {
        Self::IO
    }
}

impl From<std::io::Error> for GalileoError {
    fn from(_value: Error) -> Self {
        Self::FsIo
    }
}

#[cfg(target_arch = "wasm32")]
impl From<wasm_bindgen::JsValue> for GalileoError {
    fn from(value: wasm_bindgen::JsValue) -> Self {
        GalileoError::Wasm(Some(format!("{value:?}")))
    }
}

#[cfg(target_arch = "wasm32")]
impl From<web_sys::Element> for GalileoError {
    fn from(value: web_sys::Element) -> Self {
        GalileoError::Wasm(Some(format!("Failed to cast {value:?} into target type")))
    }
}
#[cfg(target_arch = "wasm32")]
impl From<js_sys::Object> for GalileoError {
    fn from(value: js_sys::Object) -> Self {
        GalileoError::Wasm(Some(format!("Failed to cast {value:?} into target type")))
    }
}
