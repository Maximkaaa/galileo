use crate::layer::feature_layer::symbol::Symbol;
use crate::primitives::Color;
use crate::render::{PointPaint, PrimitiveId, RenderBundle, UnpackedBundle};
use galileo_types::cartesian::impls::point::Point3d;
use galileo_types::cartesian::traits::cartesian_point::CartesianPoint2d;
use galileo_types::geometry::Geom;
use galileo_types::geometry_type::{CartesianSpace2d, CartesianSpace3d};
use galileo_types::multi_point::MultiPoint;
use nalgebra::Point3;
use std::marker::PhantomData;

pub struct CirclePointSymbol<Space> {
    pub color: Color,
    pub size: f64,
    space: PhantomData<Space>,
}

impl<Space> CirclePointSymbol<Space> {
    pub fn new(color: Color, size: f64) -> Self {
        Self {
            color,
            size,
            space: Default::default(),
        }
    }

    fn render_internal(
        &self,
        geometry: &Point3d,
        bundle: &mut Box<dyn RenderBundle>,
    ) -> Vec<PrimitiveId> {
        let paint = PointPaint {
            color: self.color,
            size: self.size,
        };
        vec![bundle.add_point(geometry, paint)]
    }
}

impl<T> Symbol<T, Point3<f64>> for CirclePointSymbol<CartesianSpace3d> {
    fn render(
        &self,
        _feature: &T,
        geometry: &Point3<f64>,
        bundle: &mut Box<dyn RenderBundle>,
    ) -> Vec<PrimitiveId> {
        self.render_internal(geometry, bundle)
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

impl<T> Symbol<T, Geom<Point3d>> for CirclePointSymbol<CartesianSpace3d> {
    fn render(
        &self,
        feature: &T,
        geometry: &Geom<Point3d>,
        bundle: &mut Box<dyn RenderBundle>,
    ) -> Vec<PrimitiveId> {
        match geometry {
            Geom::Point(p) => self.render(feature, p, bundle),
            Geom::MultiPoint(points) => points
                .iter_points()
                .flat_map(|p| self.render_internal(p, bundle))
                .collect(),
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

impl<T, P> Symbol<T, Geom<P>> for CirclePointSymbol<CartesianSpace2d>
where
    P: CartesianPoint2d<Num = f64>,
{
    fn render(
        &self,
        _feature: &T,
        geometry: &Geom<P>,
        bundle: &mut Box<dyn RenderBundle>,
    ) -> Vec<PrimitiveId> {
        match geometry {
            Geom::Point(p) => self.render_internal(&Point3d::new(p.x(), p.y(), 0.0), bundle),
            Geom::MultiPoint(points) => points
                .iter_points()
                .flat_map(|p| self.render_internal(&Point3d::new(p.x(), p.y(), 0.0), bundle))
                .collect(),
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
