use galileo_types::cartesian::Point3d;
use galileo_types::geometry::Geom;
use galileo_types::{MultiPolygon, Polygon};

use crate::layer::feature_layer::symbol::Symbol;
use crate::render::render_bundle::RenderBundle;
use crate::render::{LineCap, LinePaint, PolygonPaint};
use crate::Color;

/// Renders a polygon geometry as a filled polygon with an outline.
#[derive(Debug, Clone, Copy)]
pub struct SimplePolygonSymbol {
    /// Color of the inner area of the polygon.
    pub fill_color: Color,
    /// Color of the outline.
    pub stroke_color: Color,
    /// Width of the outline in pixels.
    pub stroke_width: f64,
    /// Offset of the outline in pixels. Positive offset will move outline outside of the polygon, negative offset
    /// will move the outline inside the polygon.
    pub stroke_offset: f64,
}

impl SimplePolygonSymbol {
    /// Creates a new instance.
    pub fn new(fill_color: Color) -> Self {
        Self {
            fill_color,
            stroke_color: Default::default(),
            stroke_width: 0.0,
            stroke_offset: 0.0,
        }
    }

    /// Creates a new instance from a copy of the current, but with the given stroke color.
    pub fn with_stroke_color(&self, stroke_color: Color) -> Self {
        Self {
            stroke_color,
            ..*self
        }
    }

    /// Creates a new instance from a copy of the current, but with the given stroke width.
    pub fn with_stroke_width(&self, stroke_width: f64) -> Self {
        Self {
            stroke_width,
            ..*self
        }
    }

    /// Creates a new instance from a copy of the current, but with the given stroke offset.
    pub fn with_stroke_offset(&self, stroke_offset: f64) -> Self {
        Self {
            stroke_offset,
            ..*self
        }
    }

    fn render_poly(
        &self,
        polygon: &galileo_types::impls::Polygon<Point3d>,
        min_resolution: f64,
        bundle: &mut RenderBundle,
    ) {
        if !self.fill_color.is_transparent() {
            bundle.add_polygon(
                polygon,
                &PolygonPaint {
                    color: self.fill_color,
                },
                min_resolution,
            );
        }

        if !self.stroke_color.is_transparent() && self.stroke_width > 0.0 {
            let line_paint = LinePaint {
                color: self.stroke_color,
                width: self.stroke_width,
                offset: self.stroke_offset,
                line_cap: LineCap::Butt,
            };

            for contour in polygon.iter_contours() {
                bundle.add_line(contour, &line_paint, min_resolution);
            }
        }
    }
}

impl<F> Symbol<F> for SimplePolygonSymbol {
    fn render(
        &self,
        _feature: &F,
        geometry: &Geom<Point3d>,
        min_resolution: f64,
        bundle: &mut RenderBundle,
    ) {
        match geometry {
            Geom::Polygon(poly) => self.render_poly(poly, min_resolution, bundle),
            Geom::MultiPolygon(polygons) => polygons
                .polygons()
                .for_each(|polygon| self.render_poly(polygon, min_resolution, bundle)),
            _ => {}
        }
    }
}
