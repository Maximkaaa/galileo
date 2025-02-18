use galileo_types::cartesian::Point3d;
use galileo_types::geometry::Geom;
use galileo_types::MultiContour;

use crate::layer::feature_layer::symbol::Symbol;
use crate::render::render_bundle::RenderBundle;
use crate::render::{LineCap, LinePaint};
use crate::Color;

/// Renders a contour as a line of fixed width.
#[derive(Debug, Copy, Clone)]
pub struct SimpleContourSymbol {
    /// Color of the line.
    pub color: Color,
    /// Width of the line in pixels.
    pub width: f64,
}

impl SimpleContourSymbol {
    /// Creates a new instance.
    pub fn new(color: Color, width: f64) -> Self {
        Self { color, width }
    }
}

impl<F> Symbol<F> for SimpleContourSymbol {
    fn render(
        &self,
        _feature: &F,
        geometry: &Geom<Point3d>,
        min_resolution: f64,
        bundle: &mut RenderBundle,
    ) {
        let paint = LinePaint {
            color: self.color,
            width: self.width,
            offset: 0.0,
            line_cap: LineCap::Butt,
        };

        match geometry {
            Geom::Contour(contour) => {
                bundle.add_line(contour, &paint, min_resolution);
            }
            Geom::MultiContour(contours) => {
                contours.contours().for_each(|contour| {
                    bundle.add_line(contour, &paint, min_resolution);
                });
            }
            _ => {}
        }
    }
}
