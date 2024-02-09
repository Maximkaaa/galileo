use crate::layer::feature_layer::symbol::Symbol;
use crate::render::render_bundle::RenderPrimitive;
use crate::render::{LineCap, LinePaint, PolygonPaint};
use crate::Color;
use galileo_types::cartesian::impls::contour::Contour;
use galileo_types::cartesian::impls::polygon::Polygon;
use galileo_types::cartesian::traits::cartesian_point::CartesianPoint3d;
use galileo_types::geometry::Geom;
use galileo_types::multi_polygon::MultiPolygon;
use num_traits::AsPrimitive;

#[derive(Debug, Clone, Copy)]
pub struct SimplePolygonSymbol {
    pub fill_color: Color,
    pub stroke_color: Color,
    pub stroke_width: f64,
    pub stroke_offset: f64,
}

impl SimplePolygonSymbol {
    pub fn new(fill_color: Color) -> Self {
        Self {
            fill_color,
            stroke_color: Default::default(),
            stroke_width: 0.0,
            stroke_offset: 0.0,
        }
    }

    pub fn with_stroke_color(&self, stroke_color: Color) -> Self {
        Self {
            stroke_color,
            ..*self
        }
    }

    pub fn with_stroke_width(&self, stroke_width: f64) -> Self {
        Self {
            stroke_width,
            ..*self
        }
    }

    pub fn with_stroke_offset(&self, stroke_offset: f64) -> Self {
        Self {
            stroke_offset,
            ..*self
        }
    }

    fn render_poly<'a, N, P>(
        &self,
        polygon: &'a Polygon<P>,
    ) -> Vec<RenderPrimitive<'a, N, P, Contour<P>, Polygon<P>>>
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
    ) -> Vec<RenderPrimitive<'a, N, P, Contour<P>, Polygon<P>>>
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
