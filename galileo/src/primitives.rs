use image::GenericImageView;
use maybe_sync::{MaybeSend, MaybeSync};
use std::any::Any;
use std::ops::Deref;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub use galileo_types::{ClosedContour, Contour, Point2d, Polygon};

#[derive(Debug, Clone, Copy, Default)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl From<winit::dpi::PhysicalSize<u32>> for Size {
    fn from(value: winit::dpi::PhysicalSize<u32>) -> Self {
        Self {
            width: value.width,
            height: value.height,
        }
    }
}

pub trait Image: MaybeSend + MaybeSync {
    fn as_any(&self) -> &dyn Any;
}

#[derive(Debug, Clone)]
pub struct DecodedImage {
    pub bytes: Vec<u8>,
    pub dimensions: (u32, u32),
}

impl DecodedImage {
    pub fn new(bytes: &[u8]) -> Self {
        let decoded = image::load_from_memory(bytes).unwrap();
        let bytes = decoded.to_rgba8();
        let dimensions = decoded.dimensions();

        Self {
            bytes: Vec::from(bytes.deref()),
            dimensions,
        }
    }
}

#[derive(Debug, Copy, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(from = "String", into = "String"))]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl From<String> for Color {
    fn from(value: String) -> Self {
        Self::try_from_hex(&value).unwrap_or(Color::rgba(0, 0, 0, 255))
    }
}

impl Into<String> for Color {
    fn into(self) -> String {
        self.to_hex()
    }
}

impl Color {
    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn to_f32_array(&self) -> [f32; 4] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        ]
    }

    pub fn to_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
    }

    pub fn try_from_hex(hex_string: &str) -> Option<Self> {
        if hex_string.len() != 7 && hex_string.len() != 9 || hex_string.chars().next()? != '#' {
            return None;
        }

        let r = u8::from_str_radix(&hex_string[1..3], 16).ok()?;
        let g = u8::from_str_radix(&hex_string[3..5], 16).ok()?;
        let b = u8::from_str_radix(&hex_string[5..7], 16).ok()?;
        let a = if hex_string.len() == 9 {
            u8::from_str_radix(&hex_string[7..9], 16).ok()?
        } else {
            255
        };

        Some(Self { r, g, b, a })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_serialization() {
        let hex = "#FF1000AA";
        let color = Color::try_from_hex(hex).unwrap();
        assert_eq!(&color.to_hex(), hex);
    }
}
