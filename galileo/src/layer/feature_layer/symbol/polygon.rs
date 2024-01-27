use crate::layer::feature_layer::symbol::Symbol;
use crate::render::render_bundle::RenderBundle;
use crate::render::{LineCap, LinePaint, PolygonPaint, PrimitiveId};
use crate::Color;
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

    fn render_poly<N: AsPrimitive<f32>, P: CartesianPoint3d<Num = N>>(
        &self,
        polygon: &Polygon<P>,
        bundle: &mut RenderBundle,
        min_resolution: f64,
    ) -> Vec<PrimitiveId> {
        let mut ids = vec![];
        let id = bundle.add_polygon(
            polygon,
            PolygonPaint {
                color: self.fill_color,
            },
            min_resolution,
        );

        ids.push(id);

        let line_paint = LinePaint {
            color: self.stroke_color,
            width: self.stroke_width,
            offset: self.stroke_offset,
            line_cap: LineCap::Butt,
        };

        for contour in polygon.iter_contours() {
            ids.push(bundle.add_line(contour, line_paint, min_resolution));
        }

        ids
    }

    fn update_internal(&self, renders_ids: &[PrimitiveId], bundle: &mut RenderBundle) {
        let poly_paint = PolygonPaint {
            color: self.fill_color,
        };

        bundle.modify_polygon(renders_ids[0], poly_paint).unwrap();

        let line_paint = LinePaint {
            color: self.stroke_color,
            width: self.stroke_width,
            offset: 0.0,
            line_cap: LineCap::Butt,
        };
        for line_id in &renders_ids[1..] {
            bundle.modify_line(*line_id, line_paint).unwrap();
        }
    }
}

impl<F> Symbol<F> for SimplePolygonSymbol {
    fn render<N: AsPrimitive<f32>, P: CartesianPoint3d<Num = N>>(
        &self,
        _feature: &F,
        geometry: &Geom<P>,
        bundle: &mut RenderBundle,
        min_resolution: f64,
    ) -> Vec<PrimitiveId> {
        match geometry {
            Geom::Polygon(poly) => self.render_poly(poly, bundle, min_resolution),
            Geom::MultiPolygon(polygons) => polygons
                .polygons()
                .flat_map(|polygon| self.render_poly(polygon, bundle, min_resolution))
                .collect(),
            _ => vec![],
        }
    }

    fn update(&self, _feature: &F, renders_ids: &[PrimitiveId], bundle: &mut RenderBundle) {
        self.update_internal(renders_ids, bundle)
    }
}
