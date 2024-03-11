use crate::render::text::{FontServiceProvider, TextShaping, TextStyle};
use cosmic_text::rustybuzz::ttf_parser::FaceParsingError;
use maybe_sync::{MaybeSend, MaybeSync};
use nalgebra::Vector2;
use static_init::dynamic;
use std::sync::{Arc, RwLock};
use thiserror::Error;

#[dynamic]
static INSTANCE: Arc<RwLock<FontService>> = Arc::new(RwLock::new(FontService::default()));

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
        &mut self,
        text: &str,
        style: &TextStyle,
        offset: Vector2<f32>,
    ) -> Result<TextShaping, FontServiceError> {
        self.provider.shape(text, style, offset)
    }
}
