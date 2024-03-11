use crate::Color;
use nalgebra::Vector2;
use serde::{Deserialize, Serialize};

mod error;
pub mod font_service;

use crate::render::text::font_service::FontServiceError;
pub use error::TextShapingError;
pub(crate) use font_service::FontService;

#[cfg(feature = "cosmic-text")]
mod cosmic_text;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextStyle {
    pub font_name: String,
    pub font_size: f32,
    #[serde(default = "default_font_color")]
    pub font_color: Color,
    #[serde(default)]
    pub horizontal_alignment: HorizontalAlignment,
    #[serde(default)]
    pub vertical_alignment: VerticalAlignment,
}

fn default_font_color() -> Color {
    Color::BLACK
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub enum HorizontalAlignment {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub enum VerticalAlignment {
    Top,
    Middle,
    #[default]
    Bottom,
}

pub(crate) enum TextShaping {
    Tessellation { glyphs: Vec<TessellatedGlyph> },
    Raster,
}

pub(crate) struct TessellatedGlyph {
    pub vertices: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

pub trait FontServiceProvider {
    fn shape(
        &mut self,
        text: &str,
        style: &TextStyle,
        offset: Vector2<f32>,
    ) -> Result<TextShaping, FontServiceError>;
}
