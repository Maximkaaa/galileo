//! Service for text rendering.

use std::sync::{Arc, OnceLock};

use galileo_types::cartesian::Vector2;
use parking_lot::RwLock;
use rustybuzz::ttf_parser::FaceParsingError;
use thiserror::Error;

use super::font_provider::FontProvider;
use crate::render::text::font_provider::DefaultFontProvider;
use crate::render::text::{TextRasterizer, TextShaping, TextStyle};

static INSTANCE: OnceLock<TextService> = OnceLock::new();

/// Error from a font service
#[derive(Debug, Error, Clone)]
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
pub struct TextService {
    pub(crate) rasterizer: RwLock<Box<dyn TextRasterizer + Send + Sync>>,
    font_provider: Box<dyn FontProvider + Send + Sync>,
}

impl TextService {
    /// Initializes the font service with the given provider.
    pub fn initialize(provider: impl TextRasterizer + Send + Sync + 'static) -> &'static Self {
        if INSTANCE.get().is_some() {
            log::warn!(
                "Font service is already initialized. Second initialization call is ignored."
            );
        }

        INSTANCE.get_or_init(|| {
            log::debug!("Initializing FontService");

            Self {
                rasterizer: RwLock::new(Box::new(provider)),
                font_provider: Box::new(DefaultFontProvider::new()),
            }
        })
    }

    /// Returns static instance of the service if it was initialized.
    pub fn instance() -> Option<&'static Self> {
        INSTANCE.get()
    }

    /// Shape the given text input with the given style.
    pub fn shape(
        text: &str,
        style: &TextStyle,
        offset: Vector2<f32>,
        dpi_scale_factor: f32,
    ) -> Result<TextShaping, FontServiceError> {
        let Some(service) = Self::instance() else {
            return Err(FontServiceError::NotInitialized);
        };

        service.rasterizer.read().shape(
            text,
            style,
            offset,
            &*service.font_provider,
            dpi_scale_factor,
        )
    }

    /// Load all fonts from the given directory (recursevly).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_fonts(&self, folder_path: impl AsRef<std::path::Path>) {
        self.font_provider
            .load_fonts_folder(folder_path.as_ref().into());
    }

    /// Loads the font faces from the given font binary data.
    pub fn load_font(&self, font_data: Arc<Vec<u8>>) {
        self.load_font_internal(font_data, true);
    }

    pub(crate) fn load_font_internal(&self, font_data: Arc<Vec<u8>>, notify_workers: bool) {
        self.font_provider.load_font_data(font_data.clone());

        if notify_workers {
            #[cfg(target_arch = "wasm32")]
            crate::async_runtime::spawn(async {
                crate::platform::web::web_workers::WebWorkerService::instance()
                    .load_font(font_data)
                    .await
            });
        }
    }
}
