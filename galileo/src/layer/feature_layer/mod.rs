use crate::layer::feature_layer::feature::Feature;
use crate::layer::feature_layer::symbol::Symbol;
use crate::layer::Layer;
use crate::messenger::Messenger;
use crate::render::wgpu::WgpuRenderer;
use crate::render::{Canvas, PackedBundle, PrimitiveId, Renderer};
use crate::view::MapView;
use galileo_types::cartesian::impls::point::{Point2d, Point3d};
use galileo_types::cartesian::traits::cartesian_point::CartesianPoint2d;
use galileo_types::geo::crs::Crs;
use galileo_types::geo::impls::point::GeoPoint2d;
use galileo_types::geo::impls::projection::identity::IdentityProjection;
use galileo_types::geo::traits::projection::{ChainProjection, InvertedProjection, Projection};
use galileo_types::geometry::{CartesianGeometry2d, Geom, Geometry};
use maybe_sync::{MaybeSend, MaybeSync};
use std::any::Any;
use std::sync::{Arc, RwLock};

pub mod feature;
pub mod symbol;

pub struct FeatureLayer<P, F, S>
where
    F: Feature,
    F::Geom: Geometry<Point = P>,
{
    features: Vec<F>,
    style: S,
    render_bundle: RwLock<Option<Box<dyn PackedBundle>>>,
    feature_render_map: RwLock<Vec<Vec<PrimitiveId>>>,
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

    pub fn crs(&self) -> &Crs {
        &self.crs
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

impl<F, S> Layer for FeatureLayer<GeoPoint2d, F, S>
where
    F: Feature + MaybeSend + MaybeSync,
    F::Geom: Geometry<Point = GeoPoint2d>,
    S: Symbol<F, Geom<Point2d>> + MaybeSend + MaybeSync,
{
    fn render(&self, position: &MapView, canvas: &mut dyn Canvas) {
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

        canvas.draw_bundles(&[&**self.render_bundle.read().unwrap().as_ref().unwrap()]);
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
    fn render(&self, view: &MapView, canvas: &mut dyn Canvas) {
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

        canvas.draw_bundles(&[&**self.render_bundle.read().unwrap().as_ref().unwrap()]);
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

impl<F, S> Layer for FeatureLayer<Point3d, F, S>
where
    F: Feature + MaybeSend + MaybeSync,
    F::Geom: Geometry<Point = Point3d>,
    S: Symbol<F, F::Geom> + MaybeSend + MaybeSync,
{
    fn render(&self, view: &MapView, canvas: &mut dyn Canvas) {
        if view.crs() != &self.crs {
            // not supported at the moment for 3d coordiantes
            return;
        }

        if self.render_bundle.read().unwrap().is_none() {
            let mut bundle = canvas.create_bundle();
            let mut render_map = self.feature_render_map.write().unwrap();

            for feature in &self.features {
                let ids = self.style.render(feature, feature.geometry(), &mut bundle);
                render_map.push(ids)
            }

            let packed = canvas.pack_bundle(bundle);
            *self.render_bundle.write().unwrap() = Some(packed);
        }

        canvas.draw_bundles(&[&**self.render_bundle.read().unwrap().as_ref().unwrap()]);
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
