use crate::layer::feature_layer::feature::Feature;
use crate::layer::feature_layer::symbol::Symbol;
use crate::layer::Layer;
use crate::messenger::Messenger;
use crate::render::wgpu::WgpuRenderer;
use crate::render::{Canvas, PackedBundle, PrimitiveId, RenderOptions, Renderer};
use crate::view::MapView;
use galileo_types::cartesian::impls::point::{Point2d, Point3d};
use galileo_types::cartesian::traits::cartesian_point::{
    CartesianPoint2d, NewCartesianPoint2d, NewCartesianPoint3d,
};
use galileo_types::geo::crs::Crs;
use galileo_types::geo::impls::point::GeoPoint2d;
use galileo_types::geo::impls::projection::dimensions::AddDimensionProjection;
use galileo_types::geo::impls::projection::identity::IdentityProjection;
use galileo_types::geo::traits::point::NewGeoPoint;
use galileo_types::geo::traits::projection::{ChainProjection, InvertedProjection, Projection};
use galileo_types::geometry::{CartesianGeometry2d, Geom, Geometry};
use galileo_types::geometry_type::{CartesianSpace2d, CartesianSpace3d, GeoSpace2d};
use maybe_sync::{MaybeSend, MaybeSync};
use num_traits::AsPrimitive;
use std::marker::PhantomData;
use std::sync::{Arc, RwLock};

pub mod feature;
pub mod symbol;

pub struct FeatureLayer<P, F, S, Space>
where
    F: Feature,
    F::Geom: Geometry<Point = P>,
{
    features: Vec<F>,
    symbol: S,
    crs: Crs,
    lods: Vec<Lod>,
    messenger: RwLock<Option<Box<dyn Messenger>>>,

    space: PhantomData<Space>,
}

struct Lod {
    min_resolution: f64,
    render_bundle: RwLock<Option<Box<dyn PackedBundle>>>,
    feature_render_map: RwLock<Vec<Vec<PrimitiveId>>>,
}

impl<P, F, S, Space> FeatureLayer<P, F, S, Space>
where
    F: Feature,
    F::Geom: Geometry<Point = P>,
{
    pub fn new(features: Vec<F>, style: S, crs: Crs) -> Self {
        Self {
            features,
            symbol: style,
            crs,
            messenger: RwLock::new(None),
            lods: vec![Lod {
                min_resolution: 1.0,
                render_bundle: RwLock::new(None),
                feature_render_map: RwLock::new(Vec::new()),
            }],
            space: Default::default(),
        }
    }

    pub fn with_lods(features: Vec<F>, style: S, crs: Crs, lods: &[f64]) -> Self {
        let mut lods: Vec<_> = lods
            .iter()
            .map(|&min_resolution| Lod {
                min_resolution,
                render_bundle: RwLock::new(None),
                feature_render_map: RwLock::new(Vec::new()),
            })
            .collect();
        lods.sort_by(|a, b| b.min_resolution.total_cmp(&a.min_resolution));

        Self {
            features,
            symbol: style,
            crs,
            messenger: RwLock::new(None),
            lods,
            space: Default::default(),
        }
    }
}

impl<P, F, S, Space> FeatureLayer<P, F, S, Space>
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

impl<P, F, S, Space> FeatureLayer<P, F, S, Space>
where
    F: Feature,
    F::Geom: Geometry<Point = P>,
    S: Symbol<F>,
{
    pub fn update_features(&mut self, indices: &[usize], renderer: &dyn Renderer) {
        for lod in &self.lods {
            let mut bundle_lock = lod.render_bundle.write().unwrap();
            let Some(bundle) = bundle_lock.take() else {
                return;
            };

            let feature_render_map = lod.feature_render_map.read().unwrap();
            let mut unpacked = bundle.unpack();
            for index in indices {
                let feature = self.features.get(*index).unwrap();
                let render_ids = feature_render_map.get(*index).unwrap();
                self.symbol.update(feature, render_ids, &mut unpacked);
            }

            // todo: remove deps on wgpu
            let wgpu: &WgpuRenderer = renderer.as_any().downcast_ref().unwrap();
            *bundle_lock = Some(wgpu.pack_bundle(unpacked));

            if let Some(messenger) = &(*self.messenger.read().unwrap()) {
                messenger.request_redraw();
            }
        }
    }

    fn select_lod(&self, resolution: f64) -> &Lod {
        debug_assert!(!self.lods.is_empty());

        for lod in &self.lods {
            if lod.min_resolution < resolution {
                return lod;
            }
        }

        &self.lods[self.lods.len() - 1]
    }
}

impl<P, F, S> Layer for FeatureLayer<P, F, S, GeoSpace2d>
where
    P: NewGeoPoint + 'static,
    F: Feature + MaybeSend + MaybeSync,
    F::Geom: Geometry<Point = P>,
    S: Symbol<F> + MaybeSend + MaybeSync,
{
    fn render(&self, view: &MapView, canvas: &mut dyn Canvas) {
        let lod = self.select_lod(view.resolution());
        if lod.render_bundle.read().unwrap().is_none() {
            let mut bundle = canvas.create_bundle();
            let mut render_map = lod.feature_render_map.write().unwrap();
            let projection = ChainProjection::new(
                view.crs().get_projection::<P, Point2d>().unwrap(),
                Box::new(AddDimensionProjection::new(0.0)),
            );

            for feature in &self.features {
                let Some(projected): Option<Geom<Point3d>> =
                    feature.geometry().project(&projection)
                else {
                    continue;
                };
                let ids = self
                    .symbol
                    .render(feature, &projected, &mut bundle, lod.min_resolution);
                render_map.push(ids)
            }

            let packed = canvas.pack_bundle(bundle);
            *lod.render_bundle.write().unwrap() = Some(packed);
        }

        canvas.draw_bundles(
            &[&**lod.render_bundle.read().unwrap().as_ref().unwrap()],
            RenderOptions::default(),
        );
    }

    fn prepare(&self, _view: &MapView, _renderer: &Arc<RwLock<dyn Renderer>>) {
        // do nothing
    }

    fn set_messenger(&mut self, messenger: Box<dyn Messenger>) {
        *self.messenger.write().unwrap() = Some(messenger);
    }
}

impl<P, F, S> Layer for FeatureLayer<P, F, S, CartesianSpace2d>
where
    P: NewCartesianPoint2d + Clone + 'static,
    F: Feature + MaybeSend + MaybeSync,
    F::Geom: Geometry<Point = P>,
    S: Symbol<F> + MaybeSend + MaybeSync,
{
    fn render(&self, view: &MapView, canvas: &mut dyn Canvas) {
        let lod = self.select_lod(view.resolution());

        if lod.render_bundle.read().unwrap().is_none() {
            let mut bundle = canvas.create_bundle();
            let mut render_map = lod.feature_render_map.write().unwrap();

            let projection: Box<dyn Projection<InPoint = _, OutPoint = Point3d>> =
                if view.crs() == &self.crs {
                    Box::new(AddDimensionProjection::new(0.0))
                } else {
                    let self_proj = self.crs.get_projection::<GeoPoint2d, P>().unwrap();
                    let view_proj: Box<dyn Projection<InPoint = _, OutPoint = Point2d>> =
                        view.crs().get_projection().unwrap();
                    Box::new(ChainProjection::new(
                        Box::new(ChainProjection::new(
                            Box::new(InvertedProjection::new(self_proj)),
                            view_proj,
                        )),
                        Box::new(AddDimensionProjection::new(0.0)),
                    ))
                };

            for feature in &self.features {
                let Some(geom) = feature.geometry().project(&*projection) else {
                    continue;
                };
                let ids = self
                    .symbol
                    .render(feature, &geom, &mut bundle, lod.min_resolution);
                render_map.push(ids)
            }

            let packed = canvas.pack_bundle(bundle);
            *lod.render_bundle.write().unwrap() = Some(packed);
        }

        canvas.draw_bundles(
            &[&**lod.render_bundle.read().unwrap().as_ref().unwrap()],
            RenderOptions::default(),
        );
    }

    fn prepare(&self, _view: &MapView, _renderer: &Arc<RwLock<dyn Renderer>>) {
        // do nothing
    }

    fn set_messenger(&mut self, messenger: Box<dyn Messenger>) {
        *self.messenger.write().unwrap() = Some(messenger);
    }
}

impl<P, F, S> Layer for FeatureLayer<P, F, S, CartesianSpace3d>
where
    P: NewCartesianPoint3d,
    P::Num: AsPrimitive<f32>,
    F: Feature + MaybeSend + MaybeSync,
    F::Geom: Geometry<Point = P>,
    S: Symbol<F> + MaybeSend + MaybeSync,
{
    fn render(&self, view: &MapView, canvas: &mut dyn Canvas) {
        if view.crs() != &self.crs {
            // not supported at the moment for 3d coordiantes
            return;
        }

        let lod = self.select_lod(view.resolution());

        if lod.render_bundle.read().unwrap().is_none() {
            let mut bundle = canvas.create_bundle();
            let mut render_map = lod.feature_render_map.write().unwrap();

            for feature in &self.features {
                let projection = IdentityProjection::<_, Point3d, _>::new();
                if let Some(geometry) = feature.geometry().project(&projection) {
                    let ids =
                        self.symbol
                            .render(feature, &geometry, &mut bundle, lod.min_resolution);
                    render_map.push(ids)
                } else {
                    render_map.push(vec![])
                };
            }

            let packed = canvas.pack_bundle(bundle);
            *lod.render_bundle.write().unwrap() = Some(packed);
        }

        let options = RenderOptions {
            antialias: self.symbol.use_antialiasing(),
        };

        canvas.draw_bundles(
            &[&**lod.render_bundle.read().unwrap().as_ref().unwrap()],
            options,
        );
    }

    fn prepare(&self, _view: &MapView, _renderer: &Arc<RwLock<dyn Renderer>>) {
        // do nothing
    }

    fn set_messenger(&mut self, messenger: Box<dyn Messenger>) {
        *self.messenger.write().unwrap() = Some(messenger);
    }
}
