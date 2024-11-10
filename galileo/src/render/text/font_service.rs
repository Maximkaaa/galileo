//! Service for text rendering.

use crate::render::text::rustybuzz::RustybuzzFontServiceProvider;
use crate::render::text::{FontServiceProvider, TextShaping, TextStyle};
use bytes::Bytes;
use lazy_static::lazy_static;
use nalgebra::Vector2;
use rustybuzz::ttf_parser::FaceParsingError;
use std::sync::{Arc, RwLock};
use thiserror::Error;

lazy_static! {
    static ref INSTANCE: Arc<RwLock<FontService>> = Arc::new(RwLock::new(FontService::default()));
}

/// Error from a font service
#[derive(Debug, Error)]
pub enum FontServiceError {
    /// Error parsing font face file
    #[error(transparent)]
    FaceParsingError(#[from] FaceParsingError),

    /// Font file not found
    #[error("font is not loaded")]
    FontNotFound,
}

/// Provides common access to underlying text shaping engine implementation.
pub struct FontService {
    pub(crate) provider: Box<dyn FontServiceProvider + Send + Sync>,
}

impl Default for FontService {
    fn default() -> Self {
        Self {
            provider: Box::new(RustybuzzFontServiceProvider::default()),
        }
    }
}

impl FontService {
    /// Return a singleton instance of the service.
    pub fn instance() -> Arc<RwLock<Self>> {
        INSTANCE.clone()
    }

    /// Execute the closure with the font service instance as an argument.
    pub fn with<T>(f: impl FnOnce(&Self) -> T) -> T {
        f(&INSTANCE.read().expect("lock is poisoned"))
    }

    /// Execute the closure with mutable reference to the font service instance as an argument.
    pub fn with_mut<T>(f: impl FnOnce(&mut Self) -> T) -> T {
        f(&mut INSTANCE.write().expect("lock is poisoned"))
    }

    /// Shape the given text input with the given style.
    pub fn shape(
        &self,
        text: &str,
        style: &TextStyle,
        offset: Vector2<f32>,
    ) -> Result<TextShaping, FontServiceError> {
        self.provider.shape(text, style, offset)
    }

    /// Try parse input binary data to load fonts to the font service.
    pub fn load_fonts(&mut self, fonts_data: Bytes) -> Result<(), FontServiceError> {
        self.provider.load_fonts(fonts_data)
    }
}
