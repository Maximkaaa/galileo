use galileo_mvt::error::GalileoMvtError;
use image::ImageError;
use std::io::Error;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GalileoError {
    #[error("failed to load data")]
    IO,
    #[error("failed to decode data")]
    Decoding(#[from] GalileoMvtError),
    #[error("wasm error: {0:?}")]
    Wasm(Option<String>),
    #[error("item not found")]
    NotFound,
    #[error("image decode error: {0:?}")]
    ImageDecode(#[from] ImageError),
    #[error("{0}")]
    Generic(String),
}

#[cfg(not(target_arch = "wasm32"))]
impl From<reqwest::Error> for GalileoError {
    fn from(_value: reqwest::Error) -> Self {
        Self::IO
    }
}

impl From<std::io::Error> for GalileoError {
    fn from(_value: Error) -> Self {
        Self::IO
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
