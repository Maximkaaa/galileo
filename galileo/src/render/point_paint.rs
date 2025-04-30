//! [`PointPaint`] specifies the way a point should be drawn to the map.

use std::borrow::Cow;
use std::sync::Arc;

use galileo_types::cartesian::{Point2, Size, Vector2};
use galileo_types::impls::ClosedContour;
use serde::{Deserialize, Serialize};

use crate::decoded_image::DecodedImage;
use crate::render::text::TextStyle;
use crate::render::{LineCap, LinePaint};
use crate::Color;

/// Specifies the way a point should be drawn to the map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointPaint<'a> {
    pub(crate) shape: PointShape<'a>,
    pub(crate) offset: Vector2<f32>,
}

impl<'a> PointPaint<'a> {
    /// Creates a paint that draws a circle of fixed diameter (in pixels) not dependent on map resolution.
    pub fn circle(color: Color, diameter: f32) -> Self {
        Self {
            offset: Vector2::default(),
            shape: PointShape::Circle {
                fill: color.into(),
                radius: diameter / 2.0,
                outline: None,
            },
        }
    }

    /// Creates a paint that draws a sector of a circle of fixed diameter (in pixels) not dependent on map resolution.
    pub fn sector(color: Color, diameter: f32, start_angle: f32, end_angle: f32) -> Self {
        Self {
            offset: Vector2::default(),
            shape: PointShape::Sector(SectorParameters {
                fill: color.into(),
                radius: diameter / 2.0,
                start_angle,
                end_angle,
                outline: None,
            }),
        }
    }

    /// Creates a paint that draws a square of fixed size (in pixels).
    pub fn square(color: Color, size: f32) -> Self {
        Self {
            offset: Vector2::default(),
            shape: PointShape::Square {
                fill: color,
                size,
                outline: None,
            },
        }
    }

    /// Creates a paint that draws a single one-pixel dot of given color.
    pub fn dot(color: Color) -> Self {
        Self {
            offset: Vector2::default(),
            shape: PointShape::Dot { color },
        }
    }

    /// Creates a paint that draws a given shape (in screen coordinates).
    pub fn shape(color: Color, contour: &'a ClosedContour<Point2<f32>>, scale: f32) -> Self {
        Self {
            offset: Vector2::default(),
            shape: PointShape::FreeShape {
                fill: color,
                scale,
                outline: None,
                shape: Cow::Borrowed(contour),
            },
        }
    }

    /// Creates a paint that draws given text label with the specified style.
    pub fn label(text: &'a str, style: &'a TextStyle) -> Self {
        Self {
            offset: Vector2::default(),
            shape: PointShape::Label {
                text: Cow::Borrowed(text),
                style: Cow::Borrowed(style),
            },
        }
    }

    /// Creates a paint that draws given text label with the specified style.
    pub fn label_owned(text: String, style: TextStyle) -> Self {
        Self {
            offset: Vector2::default(),
            shape: PointShape::Label {
                text: Cow::Owned(text),
                style: Cow::Owned(style),
            },
        }
    }

    /// Sets an outline for the symbol (if applicable).
    pub fn with_outline(mut self, color: Color, width: f32) -> Self {
        match &mut self.shape {
            PointShape::Circle { outline, .. }
            | PointShape::Square { outline, .. }
            | PointShape::FreeShape { outline, .. } => {
                *outline = Some(LinePaint {
                    color,
                    width: width as f64,
                    offset: 0.0,
                    line_cap: LineCap::Round,
                })
            }
            _ => {}
        }

        self
    }

    /// Sets offset of the paint.
    ///
    /// Offset is the distance in pixels from the base point the object will be drawn at. E.g.
    /// offset does not depend on the map resolution.
    ///
    /// Positive `x` values of offset move the object to the right, positive `y` values move the
    /// object towards the top of the screen.
    pub fn with_offset(mut self, offset: Vector2<f32>) -> Self {
        self.offset = offset;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub(crate) enum PointShape<'a> {
    Dot {
        color: Color,
    },
    Circle {
        fill: CircleFill,
        radius: f32,
        outline: Option<LinePaint>,
    },
    Sector(SectorParameters),
    Square {
        fill: Color,
        size: f32,
        outline: Option<LinePaint>,
    },
    FreeShape {
        fill: Color,
        scale: f32,
        outline: Option<LinePaint>,
        shape: Cow<'a, ClosedContour<Point2<f32>>>,
    },
    Label {
        text: Cow<'a, str>,
        style: Cow<'a, TextStyle>,
    },
}

/// Represents a point as a marker on the map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarkerStyle {
    /// Draws marker from the given image.
    Image {
        /// Image bitmap.
        image: Arc<DecodedImage>,
        /// Anchor point.
        anchor: Vector2<f32>,
        /// Size of the marker image in pixels. If not set, the size of the bitmap will be used.
        size: Option<Size<u32>>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) struct SectorParameters {
    pub fill: CircleFill,
    pub radius: f32,
    pub start_angle: f32,
    pub end_angle: f32,
    pub outline: Option<LinePaint>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) struct CircleFill {
    pub center_color: Color,
    pub side_color: Color,
}

impl From<Color> for CircleFill {
    fn from(value: Color) -> Self {
        Self {
            center_color: value,
            side_color: value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circle_fill_from_color() {
        let color = Color::RED;
        let fill: CircleFill = color.into();
        assert_eq!(fill.center_color, color);
        assert_eq!(fill.side_color, color);
    }
}
