use crate::layer::feature::symbol::Symbol;
use crate::primitives::Color;
use crate::render::{LineCap, LinePaint, Paint, PrimitiveId, RenderBundle, UnpackedBundle};
use galileo_types::cartesian::impls::point::Point2d;
use galileo_types::cartesian::impls::polygon::Polygon;
use galileo_types::geo::impls::projection::dimensions::AddDimensionProjection;

pub struct SimplePolygonSymbol {
    pub fill_color: Color,
    pub stroke_color: Color,
    pub stroke_width: f64,
    pub stroke_offset: f64,
}

impl Symbol<(), Polygon<Point2d>> for SimplePolygonSymbol {
    fn render(
        &self,
        _feature: &(),
        geometry: &Polygon<Point2d>,
        bundle: &mut Box<dyn RenderBundle>,
    ) -> Vec<PrimitiveId> {
        let mut ids = vec![];
        let id = bundle.add_polygon(
            geometry,
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
        for contour in geometry.iter_contours() {
            ids.push(bundle.add_line(
                &contour.project_points(&projection).unwrap().into(),
                line_paint,
                10000.0,
            ));
        }

        ids
    }

    fn update(
        &self,
        _feature: &(),
        renders_ids: &[PrimitiveId],
        bundle: &mut Box<dyn UnpackedBundle>,
    ) {
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
