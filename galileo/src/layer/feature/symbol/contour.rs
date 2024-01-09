use crate::layer::feature::symbol::Symbol;
use crate::primitives::Color;
use crate::render::{LineCap, LinePaint, PrimitiveId, RenderBundle, UnpackedBundle};
use galileo_types::cartesian::impls::contour::Contour;
use galileo_types::cartesian::impls::point::{Point2d, Point3d};
use galileo_types::geo::impls::projection::dimensions::AddDimensionProjection;

pub struct SimpleContourSymbol {
    pub color: Color,
    pub width: f64,
}

impl Symbol<(), Contour<Point2d>> for SimpleContourSymbol {
    fn render(
        &self,
        _feature: &(),
        geometry: &Contour<Point2d>,
        bundle: &mut Box<dyn RenderBundle>,
    ) -> Vec<PrimitiveId> {
        let projection = AddDimensionProjection::new(0.0);
        let id = bundle.add_line(
            &geometry.project_points(&projection).unwrap(),
            LinePaint {
                color: self.color,
                width: self.width,
                offset: 0.0,
                line_cap: LineCap::Butt,
            },
            10000.0,
        );

        vec![id]
    }

    fn update(
        &self,
        _feature: &(),
        _renders_ids: &[PrimitiveId],
        _bundle: &mut Box<dyn UnpackedBundle>,
    ) {
        todo!()
    }
}

impl<F> Symbol<F, Contour<Point3d>> for SimpleContourSymbol {
    fn render(
        &self,
        _feature: &F,
        geometry: &Contour<Point3d>,
        bundle: &mut Box<dyn RenderBundle>,
    ) -> Vec<PrimitiveId> {
        let id = bundle.add_line(
            &geometry,
            LinePaint {
                color: self.color,
                width: self.width,
                offset: 0.0,
                line_cap: LineCap::Butt,
            },
            10000.0,
        );

        vec![id]
    }

    fn update(
        &self,
        _feature: &F,
        _renders_ids: &[PrimitiveId],
        _bundle: &mut Box<dyn UnpackedBundle>,
    ) {
        todo!()
    }
}
