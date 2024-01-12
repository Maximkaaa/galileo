use crate::layer::feature_layer::symbol::Symbol;
use crate::primitives::Color;
use crate::render::{LineCap, LinePaint, Paint, PrimitiveId, RenderBundle, UnpackedBundle};
use galileo_types::cartesian::impls::point::Point2d;
use galileo_types::cartesian::impls::polygon::Polygon;
use galileo_types::geo::impls::projection::dimensions::AddDimensionProjection;
use galileo_types::geometry::Geom;
use galileo_types::geometry_type::CartesianSpace2d;
use galileo_types::multi_polygon::MultiPolygon;
use std::marker::PhantomData;

#[derive(Debug, Clone, Copy)]
pub struct SimplePolygonSymbol<Space> {
    pub fill_color: Color,
    pub stroke_color: Color,
    pub stroke_width: f64,
    pub stroke_offset: f64,
    space: PhantomData<Space>,
}

impl<Space> SimplePolygonSymbol<Space> {
    pub fn new(fill_color: Color) -> Self {
        Self {
            fill_color,
            stroke_color: Default::default(),
            stroke_width: 0.0,
            stroke_offset: 0.0,
            space: Default::default(),
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

    fn render_internal(
        &self,
        polygon: &Polygon<Point2d>,
        bundle: &mut Box<dyn RenderBundle>,
    ) -> Vec<PrimitiveId> {
        let mut ids = vec![];
        let id = bundle.add_polygon(
            polygon,
            Paint {
                color: self.fill_color,
            },
            10000.0,
        );

        ids.push(id);

        let line_paint = LinePaint {
            color: self.stroke_color,
            width: self.stroke_width,
            offset: self.stroke_offset,
            line_cap: LineCap::Butt,
        };

        let projection = AddDimensionProjection::new(0.0);
        for contour in polygon.iter_contours() {
            ids.push(bundle.add_line(
                &contour.project_points(&projection).unwrap().into(),
                line_paint,
                1000.0,
            ));
        }

        ids
    }

    fn update_internal(&self, renders_ids: &[PrimitiveId], bundle: &mut Box<dyn UnpackedBundle>) {
        let poly_paint = Paint {
            color: self.fill_color,
        };

        bundle.modify_polygon(renders_ids[0], poly_paint);

        let line_paint = LinePaint {
            color: self.stroke_color,
            width: self.stroke_width,
            offset: 0.0,
            line_cap: LineCap::Butt,
        };
        for line_id in &renders_ids[1..] {
            bundle.modify_line(*line_id, line_paint);
        }
    }
}

impl<F> Symbol<F, Geom<Point2d>> for SimplePolygonSymbol<CartesianSpace2d> {
    fn render(
        &self,
        _feature: &F,
        geometry: &Geom<Point2d>,
        bundle: &mut Box<dyn RenderBundle>,
    ) -> Vec<PrimitiveId> {
        match geometry {
            Geom::Polygon(polygon) => self.render_internal(polygon, bundle),
            Geom::MultiPolygon(polygons) => polygons
                .polygons()
                .flat_map(|p| self.render_internal(p, bundle))
                .collect(),
            _ => vec![],
        }
    }

    fn update(
        &self,
        _feature: &F,
        renders_ids: &[PrimitiveId],
        bundle: &mut Box<dyn UnpackedBundle>,
    ) {
        self.update_internal(renders_ids, bundle)
    }
}
impl<F> Symbol<F, Polygon<Point2d>> for SimplePolygonSymbol<CartesianSpace2d> {
    fn render(
        &self,
        _feature: &F,
        geometry: &Polygon<Point2d>,
        bundle: &mut Box<dyn RenderBundle>,
    ) -> Vec<PrimitiveId> {
        self.render_internal(geometry, bundle)
    }

    fn update(
        &self,
        _feature: &F,
        renders_ids: &[PrimitiveId],
        bundle: &mut Box<dyn UnpackedBundle>,
    ) {
        self.update_internal(renders_ids, bundle)
    }
}
