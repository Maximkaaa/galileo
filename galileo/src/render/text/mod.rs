//! Types for text rendering.

use crate::Color;
use bytes::Bytes;
use nalgebra::Vector2;
use serde::{Deserialize, Serialize};

pub mod font_service;

use crate::render::text::font_service::FontServiceError;
pub(crate) use font_service::FontService;

// #[cfg(feature = "cosmic-text")]
// mod cosmic_text;

#[cfg(feature = "rustybuzz")]
mod rustybuzz;

/// Style of a text label on the map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextStyle {
    /// Name of the font to use.
    pub font_name: String,
    /// Size of the font in pixels.
    pub font_size: f32,
    /// Color of the font.
    #[serde(default = "default_font_color")]
    pub font_color: Color,
    /// Alignment of label along horizontal axis.
    #[serde(default)]
    pub horizontal_alignment: HorizontalAlignment,
    /// Alignment of label along vertical axis.
    #[serde(default)]
    pub vertical_alignment: VerticalAlignment,
}

fn default_font_color() -> Color {
    Color::BLACK
}

/// Horizontal alignment.
#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub enum HorizontalAlignment {
    /// Align to left.
    Left,
    /// Align to center.
    #[default]
    Center,
    /// Align to right.
    Right,
}

/// Vertical alignment.
#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub enum VerticalAlignment {
    /// Align to top.
    Top,
    /// Align to middle.
    Middle,
    /// Align to botton.
    #[default]
    Bottom,
}

/// Type of text render to use for label.
pub enum TextShaping {
    /// Text will be renderred as a set of tessellated glyphs (e.g. a number of triangles) and
    /// will be rasterised by GPU on each frame.
    Tessellation {
        /// Tessellation data.
        glyphs: Vec<TessellatedGlyph>,
    },
    /// Text will be renderred as a set of symbol images.
    Raster,
}

/// Tessellation of a single font glyph.
pub struct TessellatedGlyph {
    /// Vertices.
    pub vertices: Vec<[f32; 2]>,
    /// Indices.
    pub indices: Vec<u32>,
}

/// Data provider for font service.
pub trait FontServiceProvider {
    /// Shape text label.
    fn shape(
        &self,
        text: &str,
        style: &TextStyle,
        offset: Vector2<f32>,
    ) -> Result<TextShaping, FontServiceError>;

    /// Try to Load fonts from the given binary data.
    fn load_fonts(&mut self, fonts_data: Bytes) -> Result<(), FontServiceError>;
}
