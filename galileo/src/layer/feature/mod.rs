use crate::layer::feature::feature::Feature;
use crate::layer::Layer;
use crate::render::wgpu::WgpuRenderer;
use crate::render::{
    Canvas, LineCap, LinePaint, PackedBundle, Paint, PointPaint, RenderBundle, Renderer,
    UnpackedBundle,
};
use crate::view::MapView;
use galileo_types::cartesian::traits::cartesian_point::CartesianPoint2d;
use galileo_types::geo::crs::Crs;
use galileo_types::geo::impls::point::GeoPoint2d;
use galileo_types::geo::impls::projection::identity::IdentityProjection;
use galileo_types::geo::traits::projection::{ChainProjection, InvertedProjection, Projection};
use galileo_types::geometry::{CartesianGeometry2d, Geom, Geometry};
use maybe_sync::{MaybeSend, MaybeSync};
use nalgebra::Point3;

use crate::messenger::Messenger;
use crate::primitives::Color;
use galileo_types::cartesian::impls::contour::Contour;
use galileo_types::cartesian::impls::point::Point2d;
use galileo_types::cartesian::impls::polygon::Polygon;
use std::any::Any;
use std::sync::{Arc, RwLock};

pub mod feature;

pub struct FeatureLayer<P, F, S>
where
    F: Feature,
    F::Geom: Geometry<Point = P>,
{
    features: Vec<F>,
    style: S,
    render_bundle: RwLock<Option<Box<dyn PackedBundle>>>,
    feature_render_map: RwLock<Vec<Vec<usize>>>,
    crs: Crs,
    messenger: RwLock<Option<Box<dyn Messenger>>>,
}

impl<P, F, S> FeatureLayer<P, F, S>
where
    F: Feature,
    F::Geom: Geometry<Point = P>,
{
    pub fn new(features: Vec<F>, style: S, crs: Crs) -> Self {
        Self {
            features,
            style,
            render_bundle: RwLock::new(None),
            feature_render_map: RwLock::new(Vec::new()),
            crs,
            messenger: RwLock::new(None),
        }
    }
}

impl<P, F, S> FeatureLayer<P, F, S>
where
    P: CartesianPoint2d,
    F: Feature,
    F::Geom: Geometry<Point = P>,
{
    pub fn get_features_at(
        &self,
        point: &impl CartesianPoint2d<Num = P::Num>,
        tolerance: P::Num,
    ) -> Vec<(usize, &F)>
    where
        F::Geom: CartesianGeometry2d<P>,
    {
        self.features
            .iter()
            .enumerate()
            .filter(|(_, f)| f.geometry().is_point_inside(point, tolerance))
            .collect()
    }

    pub fn get_features_at_mut(
        &mut self,
        point: &impl CartesianPoint2d<Num = P::Num>,
        tolerance: P::Num,
    ) -> Vec<(usize, &mut F)>
    where
        F::Geom: CartesianGeometry2d<P>,
    {
        self.features
            .iter_mut()
            .enumerate()
            .filter(|(_, f)| f.geometry().is_point_inside(point, tolerance))
            .collect()
    }

    pub fn features(&self) -> impl Iterator + '_ {
        self.features.iter()
    }

    pub fn features_mut(&mut self) -> impl Iterator<Item = &'_ mut F> + '_ {
        self.features.iter_mut()
    }
}

impl<P, F, S> FeatureLayer<P, F, S>
where
    F: Feature,
    F::Geom: Geometry<Point = P>,
    S: Symbol<F, Geom<P>>,
{
    pub fn update_features(&mut self, indices: &[usize], renderer: &dyn Renderer) {
        let mut bundle_lock = self.render_bundle.write().unwrap();
        let Some(bundle) = bundle_lock.take() else {
            return;
        };

        let feature_render_map = self.feature_render_map.read().unwrap();
        let mut unpacked = bundle.unpack();
        for index in indices {
            let feature = self.features.get(*index).unwrap();
            let render_ids = feature_render_map.get(*index).unwrap();
            self.style.update(feature, render_ids, &mut unpacked);
        }

        // todo: remove deps on wgpu
        let wgpu: &WgpuRenderer = renderer.as_any().downcast_ref().unwrap();
        *bundle_lock = Some(wgpu.pack_bundle(unpacked));

        if let Some(messenger) = &(*self.messenger.read().unwrap()) {
            messenger.request_redraw();
        }
    }
}

pub trait Symbol<F, G> {
    fn render(&self, feature: &F, geometry: &G, bundle: &mut Box<dyn RenderBundle>) -> Vec<usize>;
    fn update(&self, feature: &F, renders_ids: &[usize], bundle: &mut Box<dyn UnpackedBundle>);
}

impl<F, S> Layer for FeatureLayer<GeoPoint2d, F, S>
where
    F: Feature + MaybeSend + MaybeSync,
    F::Geom: Geometry<Point = GeoPoint2d>,
    S: Symbol<F, Geom<Point2d>> + MaybeSend + MaybeSync,
{
    fn render<'a>(&self, position: &MapView, canvas: &'a mut dyn Canvas) {
        if self.render_bundle.read().unwrap().is_none() {
            let mut bundle = canvas.create_bundle();
            let mut render_map = self.feature_render_map.write().unwrap();
            let crs = position
                .crs()
                .get_projection::<GeoPoint2d, Point2d>()
                .unwrap();
            for feature in &self.features {
                let Some(projected) = feature.geometry().project(&(*crs)) else {
                    continue;
                };
                let ids = self.style.render(feature, &projected, &mut bundle);
                render_map.push(ids)
            }

            let packed = canvas.pack_bundle(bundle);
            *self.render_bundle.write().unwrap() = Some(packed);
        }

        canvas.draw_bundles(&[self.render_bundle.read().unwrap().as_ref().unwrap()]);
    }

    fn prepare(&self, _view: &MapView, _renderer: &Arc<RwLock<dyn Renderer>>) {
        // do nothing
    }

    fn set_messenger(&self, messenger: Box<dyn Messenger>) {
        *self.messenger.write().unwrap() = Some(messenger);
    }

    fn as_any(&self) -> &dyn Any {
        todo!()
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        todo!()
    }
}

impl<F, S> Layer for FeatureLayer<Point2d, F, S>
where
    F: Feature + MaybeSend + MaybeSync,
    F::Geom: Geometry<Point = Point2d>,
    S: Symbol<F, Geom<Point2d>> + MaybeSend + MaybeSync,
{
    fn render<'a>(&self, view: &MapView, canvas: &'a mut dyn Canvas) {
        if self.render_bundle.read().unwrap().is_none() {
            let mut bundle = canvas.create_bundle();
            let mut render_map = self.feature_render_map.write().unwrap();

            let projection: Box<dyn Projection<InPoint = _, OutPoint = _>> =
                if view.crs() == &self.crs {
                    Box::new(IdentityProjection::new())
                } else {
                    let self_proj = self.crs.get_projection::<GeoPoint2d, Point2d>().unwrap();
                    let view_proj = view.crs().get_projection().unwrap();
                    Box::new(ChainProjection::new(
                        Box::new(InvertedProjection::new(self_proj)),
                        view_proj,
                    ))
                };

            for feature in &self.features {
                let Some(geom) = feature.geometry().project(&*projection) else {
                    continue;
                };
                let ids = self.style.render(feature, &geom, &mut bundle);
                render_map.push(ids)
            }

            let packed = canvas.pack_bundle(bundle);
            *self.render_bundle.write().unwrap() = Some(packed);
        }

        canvas.draw_bundles(&[self.render_bundle.read().unwrap().as_ref().unwrap()]);
    }

    fn prepare(&self, _view: &MapView, _renderer: &Arc<RwLock<dyn Renderer>>) {
        // do nothing
    }

    fn set_messenger(&self, messenger: Box<dyn Messenger>) {
        *self.messenger.write().unwrap() = Some(messenger);
    }

    fn as_any(&self) -> &dyn Any {
        todo!()
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        todo!()
    }
}

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
        // let contour = Contour {
        //     points: [*feature, *feature].into(),
        //     is_closed: false,
        // };
        // let id = bundle.add_line(
        //     &contour,
        //     LinePaint {
        //         color: self.color,
        //         width: self.size,
        //         offset: 0.0,
        //         line_cap: LineCap::Round,
        //     },
        //     1.0,
        // );
        //
        // vec![id]
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

pub struct SimpleLineSymbol {
    pub color: Color,
    pub width: f64,
}

impl Symbol<(), Contour<Point2d>> for SimpleLineSymbol {
    fn render(
        &self,
        _feature: &(),
        geometry: &Contour<Point2d>,
        bundle: &mut Box<dyn RenderBundle>,
    ) -> Vec<usize> {
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

    fn update(&self, _feature: &(), _renders_ids: &[usize], _bundle: &mut Box<dyn UnpackedBundle>) {
        todo!()
    }
}

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
    ) -> Vec<usize> {
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

        for contour in geometry.iter_contours() {
            ids.push(bundle.add_line(&contour.clone().into(), line_paint, 10000.0));
        }

        ids
    }

    fn update(&self, _feature: &(), renders_ids: &[usize], bundle: &mut Box<dyn UnpackedBundle>) {
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
