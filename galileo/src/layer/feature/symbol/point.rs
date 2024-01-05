use crate::layer::feature::symbol::Symbol;
use crate::primitives::Color;
use crate::render::{PointPaint, RenderBundle, UnpackedBundle};
use galileo_types::cartesian::impls::point::Point2d;
use galileo_types::cartesian::traits::cartesian_point::CartesianPoint2d;
use galileo_types::geometry::Geom;
use nalgebra::Point3;

pub struct CirclePointSymbol {
    pub color: Color,
    pub size: f64,
}

impl<T> Symbol<T, Vec<Point3<f64>>> for CirclePointSymbol {
    fn render(
        &self,
        _feature: &T,
        geometry: &Vec<Point3<f64>>,
        bundle: &mut Box<dyn RenderBundle>,
    ) -> Vec<usize> {
        let paint = PointPaint {
            color: self.color,
            size: self.size,
        };
        bundle.add_points(geometry, paint);

        vec![]
    }

    fn update(&self, _feature: &T, _renders_ids: &[usize], _bundle: &mut Box<dyn UnpackedBundle>) {
        todo!()
    }
}

impl<T> Symbol<T, Geom<Point2d>> for CirclePointSymbol {
    fn render(
        &self,
        _feature: &T,
        geometry: &Geom<Point2d>,
        bundle: &mut Box<dyn RenderBundle>,
    ) -> Vec<usize> {
        let paint = PointPaint {
            color: self.color,
            size: self.size,
        };
        if let Geom::Point(p) = geometry {
            bundle.add_points(&vec![Point3::new(p.x(), p.y(), 0.0)], paint);
        }

        vec![]
    }

    fn update(&self, _feature: &T, _renders_ids: &[usize], _bundle: &mut Box<dyn UnpackedBundle>) {
        todo!()
    }
}
