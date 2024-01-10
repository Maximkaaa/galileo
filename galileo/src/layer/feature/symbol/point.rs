use crate::layer::feature::symbol::Symbol;
use crate::primitives::Color;
use crate::render::{PointPaint, PrimitiveId, RenderBundle, UnpackedBundle};
use galileo_types::cartesian::impls::point::{Point2d, Point3d};
use galileo_types::geometry::Geom;
use nalgebra::Point3;

pub struct CirclePointSymbol {
    pub color: Color,
    pub size: f64,
}

impl<T> Symbol<T, Point3<f64>> for CirclePointSymbol {
    fn render(
        &self,
        _feature: &T,
        geometry: &Point3<f64>,
        bundle: &mut Box<dyn RenderBundle>,
    ) -> Vec<PrimitiveId> {
        let paint = PointPaint {
            color: self.color,
            size: self.size,
        };
        vec![bundle.add_point(geometry, paint)]
    }

    fn update(
        &self,
        _feature: &T,
        _renders_ids: &[PrimitiveId],
        _bundle: &mut Box<dyn UnpackedBundle>,
    ) {
        todo!()
    }
}

impl<T> Symbol<T, Geom<Point3d>> for CirclePointSymbol {
    fn render(
        &self,
        feature: &T,
        geometry: &Geom<Point3d>,
        bundle: &mut Box<dyn RenderBundle>,
    ) -> Vec<PrimitiveId> {
        match geometry {
            Geom::Point(p) => self.render(feature, p, bundle),
            _ => vec![],
        }
    }

    fn update(
        &self,
        _feature: &T,
        _renders_ids: &[PrimitiveId],
        _bundle: &mut Box<dyn UnpackedBundle>,
    ) {
        todo!()
    }
}

impl<T> Symbol<T, Geom<Point2d>> for CirclePointSymbol {
    fn render(
        &self,
        feature: &T,
        geometry: &Geom<Point2d>,
        bundle: &mut Box<dyn RenderBundle>,
    ) -> Vec<PrimitiveId> {
        match geometry {
            Geom::Point(p) => self.render(feature, &Point3d::new(p.x, p.y, 0.0), bundle),
            _ => vec![],
        }
    }

    fn update(
        &self,
        _feature: &T,
        _renders_ids: &[PrimitiveId],
        _bundle: &mut Box<dyn UnpackedBundle>,
    ) {
        todo!()
    }
}
