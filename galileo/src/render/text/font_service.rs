//! Service for text rendering.

use std::sync::OnceLock;

use bytes::Bytes;
use nalgebra::Vector2;
use rustybuzz::ttf_parser::FaceParsingError;
use thiserror::Error;

use crate::render::text::{FontServiceProvider, TextShaping, TextStyle};

static INSTANCE: OnceLock<FontService> = OnceLock::new();

/// Error from a font service
#[derive(Debug, Error)]
pub enum FontServiceError {
    /// Error parsing font face file
    #[error(transparent)]
    FaceParsingError(#[from] FaceParsingError),

    /// Font file not found
    #[error("font is not loaded")]
    FontNotFound,

    /// Font service is not initialized
    #[error("font service is not initialize")]
    NotInitialized,
}

/// Provides common access to underlying text shaping engine implementation.
pub struct FontService {
    pub(crate) provider: Box<dyn FontServiceProvider + Send + Sync>,
}

impl FontService {
    /// Initailizes the font service with the given provider.
    pub fn initialize(provider: impl FontServiceProvider + Send + Sync + 'static) {
        if INSTANCE.get().is_some() {
            log::warn!(
                "Font service is already initialized. Second initialization call is ignored."
            );
        }

        INSTANCE.get_or_init(|| Self {
            provider: Box::new(provider),
        });
    }

    fn instance() -> Option<&'static Self> {
        INSTANCE.get()
    }

    /// Shape the given text input with the given style.
    pub fn shape(
        text: &str,
        style: &TextStyle,
        offset: Vector2<f32>,
    ) -> Result<TextShaping, FontServiceError> {
        let Some(service) = Self::instance() else {
            return Err(FontServiceError::NotInitialized);
        };

        service.provider.shape(text, style, offset)
    }

    /// Try parse input binary data to load fonts to the font service.
    pub fn load_fonts(&mut self, fonts_data: Bytes) -> Result<(), FontServiceError> {
        self.provider.load_fonts(fonts_data)
    }
}
