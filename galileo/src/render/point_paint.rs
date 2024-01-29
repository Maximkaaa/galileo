use crate::primitives::DecodedImage;
use crate::render::{LineCap, LinePaint};
use crate::Color;
use galileo_types::cartesian::impls::contour::ClosedContour;
use nalgebra::{Point2, Vector2};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct PointPaint<'a> {
    pub(crate) shape: PointShape<'a>,
    pub(crate) offset: Vector2<f32>,
}

impl<'a> PointPaint<'a> {
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

    pub fn dot(color: Color) -> Self {
        Self {
            offset: Vector2::default(),
            shape: PointShape::Dot { color },
        }
    }

    pub fn shape(color: Color, contour: &'a ClosedContour<Point2<f32>>, scale: f32) -> Self {
        Self {
            offset: Vector2::default(),
            shape: PointShape::FreeShape {
                fill: color,
                scale,
                outline: None,
                shape: contour,
            },
        }
    }

    pub fn image(image: Arc<DecodedImage>, offset: Vector2<f32>, scale: f32) -> Self {
        let width = image.dimensions.0 as f32 * scale;
        let height = image.dimensions.1 as f32 * scale;
        Self {
            offset,
            shape: PointShape::Image {
                image,
                opacity: 255,
                width,
                height,
            },
        }
    }

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
}

#[derive(Debug, Clone)]
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
        shape: &'a ClosedContour<Point2<f32>>,
    },
    Image {
        image: Arc<DecodedImage>,
        opacity: u8,
        width: f32,
        height: f32,
    },
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SectorParameters {
    pub fill: CircleFill,
    pub radius: f32,
    pub start_angle: f32,
    pub end_angle: f32,
    pub outline: Option<LinePaint>,
}

#[derive(Debug, Clone, Copy)]
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
