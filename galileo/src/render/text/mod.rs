//! Types for text rendering.

use bytes::Bytes;
use galileo_types::cartesian::Vector2;
use serde::{Deserialize, Serialize};

use crate::Color;

pub mod font_service;

pub(crate) use font_service::FontService;

use crate::render::text::font_service::FontServiceError;

// #[cfg(feature = "cosmic-text")]
// mod cosmic_text;

#[cfg(feature = "rustybuzz")]
mod rustybuzz;
#[cfg(feature = "rustybuzz")]
pub use rustybuzz::RustybuzzFontServiceProvider;

/// Style of a text label on the map.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextStyle {
    /// Name of the font to use.
    pub font_family: Vec<String>,
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
    /// Weight of the font.
    #[serde(default)]
    pub weight: FontWeight,
    /// sTyle of the font.
    #[serde(default)]
    pub style: FontStyle,
    /// Width of the outline around the letters.
    #[serde(default)]
    pub outline_width: f32,
    /// Color of the outline around the letters.
    #[serde(default = "default_outline_color")]
    pub outline_color: Color,
}

fn default_font_color() -> Color {
    Color::BLACK
}

fn default_outline_color() -> Color {
    Color::TRANSPARENT
}

/// Horizontal alignment.
#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
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
#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub enum VerticalAlignment {
    /// Align to top.
    Top,
    /// Align to middle.
    Middle,
    /// Align to bottom.
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

/// Vertex of a vectorized glyph
#[derive(Debug, Copy, Clone)]
pub struct GlyphVertex {
    /// Coordinate of the vertex (pixels)
    pub position: [f32; 2],
    /// Color of the vertex
    pub color: Color,
}

/// Tessellation of a single font glyph.
#[derive(Debug, Clone)]
pub struct TessellatedGlyph {
    /// Vertices.
    pub vertices: Vec<GlyphVertex>,
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

/// Font weight.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FontWeight(pub(crate) u16);

impl FontWeight {
    /// Normal font.
    pub const NORMAL: Self = FontWeight(500);
    /// Bold font.
    pub const BOLD: Self = FontWeight(700);
    /// Thin font.
    pub const THIN: Self = FontWeight(300);
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::NORMAL
    }
}

impl From<FontWeight> for font_query::Weight {
    fn from(value: FontWeight) -> Self {
        Self(value.0)
    }
}

/// Font style.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum FontStyle {
    /// Normal font.
    Normal,
    /// Italic font.
    Italic,
    /// Oblique font.
    Oblique,
}

impl Default for FontStyle {
    fn default() -> Self {
        Self::Normal
    }
}

impl From<FontStyle> for font_query::Style {
    fn from(value: FontStyle) -> Self {
        match value {
            FontStyle::Normal => Self::Normal,
            FontStyle::Italic => Self::Italic,
            FontStyle::Oblique => Self::Oblique,
        }
    }
}
