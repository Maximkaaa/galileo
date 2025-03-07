#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Color representation.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(from = "String", into = "String"))]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl From<String> for Color {
    fn from(value: String) -> Self {
        Self::try_from_hex(&value).unwrap_or(Color::rgba(0, 0, 0, 255))
    }
}

impl From<Color> for String {
    fn from(val: Color) -> Self {
        val.to_hex()
    }
}

impl Color {
    /// Transparent color: `#00000000`
    pub const TRANSPARENT: Color = Color::rgba(0, 0, 0, 0);
    /// Red color: `#FF0000FF`
    pub const RED: Color = Color::rgba(255, 0, 0, 255);
    /// Green color: `#00FF00FF`
    pub const GREEN: Color = Color::rgba(0, 255, 0, 255);
    /// Blue color: `#0000FFFF`
    pub const BLUE: Color = Color::rgba(0, 0, 255, 255);
    /// White color: `#FFFFFFFF`
    pub const WHITE: Color = Color::rgba(255, 255, 255, 255);
    /// Black color: `#000000FF`
    pub const BLACK: Color = Color::rgba(0, 0, 0, 255);
    /// Gray color: `#AAAAAAFF`
    pub const GRAY: Color = Color::rgba(170, 170, 170, 255);
    /// Purple color: `#800080FF`
    pub const PURPLE: Color = Color::rgba(128, 0, 128, 255);

    /// Constructs color from its RGBA channels.
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Converts the color into f32 array as used by wgpu.
    pub fn to_f32_array(&self) -> [f32; 4] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        ]
    }

    /// Converts the color into u8 array (RGBA).
    pub fn to_u8_array(&self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Converts the color into HEX8 string: `#RRGGBBAA`.
    pub fn to_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
    }

    /// Parses a color from the hex string. Hex string can be either HEX6 (`#RRGGBB`) or HEX8 (`#RRGGBBAA`).
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

    /// Parses a color from the hex string. Hex string can be either HEX6 (`#RRGGBB`) or HEX8 (`#RRGGBBAA`).
    ///
    /// # Panics
    ///
    /// Panics if the parsing fails.
    pub const fn from_hex(hex_string: &'static str) -> Self {
        let bytes = hex_string.as_bytes();
        if bytes.len() != 7 && bytes.len() != 9 || bytes[0] != b'#' {
            panic!("Invalid color hex string");
        }

        let r = decode_byte(&[bytes[1], bytes[2]]);
        let g = decode_byte(&[bytes[3], bytes[4]]);
        let b = decode_byte(&[bytes[5], bytes[6]]);
        let a = if hex_string.len() == 9 {
            decode_byte(&[bytes[7], bytes[8]])
        } else {
            255
        };

        Self { r, g, b, a }
    }

    /// Returns a new color instance, copied from the base one but with the given alpha channel.
    pub fn with_alpha(&self, a: u8) -> Self {
        Self { a, ..*self }
    }

    /// Returns true if the color is fully transparent (`a == 0`).
    pub fn is_transparent(&self) -> bool {
        self.a == 0
    }

    /// Red component of the color in RGBA space.
    pub fn r(&self) -> u8 {
        self.r
    }

    /// Green component of the color in RGBA space.
    pub fn g(&self) -> u8 {
        self.g
    }

    /// Blue component of the color in RGBA space.
    pub fn b(&self) -> u8 {
        self.b
    }

    /// Opacity component of the color.
    pub fn a(&self) -> u8 {
        self.a
    }

    /// Alpha blends `self` color with the given foreground one using foregraound color alpha.
    pub fn blend(&self, fore: Color) -> Color {
        let back_r = self.r as f32 / 255.0;
        let back_g = self.g as f32 / 255.0;
        let back_b = self.b as f32 / 255.0;

        let fore_r = fore.r as f32 / 255.0;
        let fore_g = fore.g as f32 / 255.0;
        let fore_b = fore.b as f32 / 255.0;

        let a = fore.a as f32 / 255.0;

        Color {
            r: ((back_r * (1.0 - a) + fore_r * a) * 255.0) as u8,
            g: ((back_g * (1.0 - a) + fore_g * a) * 255.0) as u8,
            b: ((back_b * (1.0 - a) + fore_b * a) * 255.0) as u8,
            a: self.a,
        }
    }
}

const fn decode_byte(chars: &[u8]) -> u8 {
    debug_assert!(chars.len() == 2);
    let first = decode_char(chars[0]);
    let second = decode_char(chars[1]);

    first * 16 + second
}

const fn decode_char(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        b'A'..=b'F' => byte - b'A' + 10,
        _ => panic!("Invalid hex character"),
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

        assert_eq!(Color::from_hex(hex), color);
    }
}
