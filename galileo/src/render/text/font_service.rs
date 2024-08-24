use crate::render::text::rustybuzz::RustybuzzFontServiceProvider;
use crate::render::text::{FontServiceProvider, TextShaping, TextStyle};
use bytes::Bytes;
use lazy_static::lazy_static;
use maybe_sync::{MaybeSend, MaybeSync};
use nalgebra::Vector2;
use rustybuzz::ttf_parser::FaceParsingError;
use std::sync::{Arc, RwLock};
use thiserror::Error;

lazy_static! {
    static ref INSTANCE: Arc<RwLock<FontService>> = Arc::new(RwLock::new(FontService::default()));
}

#[derive(Debug, Error)]
pub enum FontServiceError {
    #[error(transparent)]
    FaceParsingError(#[from] FaceParsingError),
    #[error("font is not loaded")]
    FontNotFound,
}

pub struct FontService {
    pub(crate) provider: Box<dyn FontServiceProvider + MaybeSync + MaybeSend>,
}

impl Default for FontService {
    fn default() -> Self {
        Self {
            provider: Box::new(RustybuzzFontServiceProvider::default()),
        }
    }
}

impl FontService {
    pub fn instance() -> Arc<RwLock<Self>> {
        INSTANCE.clone()
    }

    pub fn with<T>(f: impl FnOnce(&Self) -> T) -> T {
        f(&INSTANCE.read().expect("lock is poisoned"))
    }

    pub fn with_mut<T>(f: impl FnOnce(&mut Self) -> T) -> T {
        f(&mut INSTANCE.write().expect("lock is poisoned"))
    }

    pub fn shape(
        &self,
        text: &str,
        style: &TextStyle,
        offset: Vector2<f32>,
    ) -> Result<TextShaping, FontServiceError> {
        self.provider.shape(text, style, offset)
    }

    pub fn load_fonts(&mut self, fonts_data: Bytes) -> Result<(), FontServiceError> {
        self.provider.load_fonts(fonts_data)
    }
}
