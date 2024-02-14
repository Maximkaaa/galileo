use crate::layer::feature_layer::symbol::Symbol;
use crate::render::render_bundle::RenderPrimitive;
use crate::render::{LineCap, LinePaint, PolygonPaint};
use crate::Color;
use galileo_types::cartesian::CartesianPoint3d;
use galileo_types::geometry::Geom;
use galileo_types::impls::Contour;
use galileo_types::{MultiPolygon, Polygon};
use num_traits::AsPrimitive;

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

    fn render_poly<'a, N, P>(
        &self,
        polygon: &'a galileo_types::impls::Polygon<P>,
    ) -> Vec<RenderPrimitive<'a, N, P, Contour<P>, galileo_types::impls::Polygon<P>>>
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N> + Clone,
    {
        let mut primitives = vec![];
        primitives.push(RenderPrimitive::new_polygon_ref(
            polygon,
            PolygonPaint {
                color: self.fill_color,
            },
        ));

        let line_paint = LinePaint {
            color: self.stroke_color,
            width: self.stroke_width,
            offset: self.stroke_offset,
            line_cap: LineCap::Butt,
        };

        for contour in polygon.iter_contours() {
            primitives.push(RenderPrimitive::new_contour(
                contour.clone().into(),
                line_paint,
            ));
        }

        primitives
    }
}

impl<F> Symbol<F> for SimplePolygonSymbol {
    fn render<'a, N, P>(
        &self,
        _feature: &F,
        geometry: &'a Geom<P>,
        _min_resolution: f64,
    ) -> Vec<RenderPrimitive<'a, N, P, Contour<P>, galileo_types::impls::Polygon<P>>>
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N> + Clone,
    {
        match geometry {
            Geom::Polygon(poly) => self.render_poly(poly),
            Geom::MultiPolygon(polygons) => polygons
                .polygons()
                .flat_map(|polygon| self.render_poly(polygon))
                .collect(),
            _ => vec![],
        }
    }
}
