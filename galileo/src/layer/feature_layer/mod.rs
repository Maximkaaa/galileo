use crate::layer::feature_layer::feature::Feature;
use crate::layer::feature_layer::symbol::Symbol;
use crate::layer::Layer;
use crate::messenger::Messenger;
use crate::render::render_bundle::RenderBundle;
use crate::render::{Canvas, PackedBundle, PrimitiveId, RenderOptions};
use crate::view::MapView;
use galileo_types::cartesian::impls::point::{Point2d, Point3d};
use galileo_types::cartesian::rect::Rect;
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
use std::any::Any;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::RwLock;

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
    options: FeatureLayerOptions,

    space: PhantomData<Space>,
}

#[derive(Debug, Copy, Clone)]
pub struct FeatureLayerOptions {
    /// If set to true, images drawn by the layer will be sorted by the depth value (relative to viewer) before being
    /// rendered.
    ///
    /// This option is useful for layers that render points as images, and when the map is rendered in 3D you want the
    /// images that are positioned behind other pins to be drawn behind. Without this option, the images are drawn in
    /// the order they are added to the feature list.
    ///
    /// Use this with caution though, as turning on this option affects performance drastically. You probably don't want
    /// it if the layer will have more then a 1000 images drawn. If you decide to use this option for larger layers
    /// anyway, don't forget to also increase [`buffer_size_limit`](FeatureLayerOptions::buffer_size_limit) as only
    /// features from the same buffer will be sorted.
    pub sort_by_depth: bool,

    /// Sets up a soft limit on the internal GPU buffers' size (in bytes) used to render this layer. Larger values
    /// slightly improve performance when rendering, bun drastically improve performance when updating just a
    /// few features from the set.
    pub buffer_size_limit: usize,
}

impl Default for FeatureLayerOptions {
    fn default() -> Self {
        Self {
            sort_by_depth: false,
            buffer_size_limit: 10_000_000,
        }
    }
}

struct Lod {
    min_resolution: f64,
    render_bundles: RwLock<Vec<RenderBundle>>,
    packed_bundles: RwLock<Vec<Option<Box<dyn PackedBundle>>>>,
    feature_render_map: RwLock<Vec<RenderMapEntry>>,
}

struct RenderMapEntry {
    bundle_index: usize,
    primitive_ids: Vec<PrimitiveId>,
}

impl<P, F, S, Space> FeatureLayer<P, F, S, Space>
where
    F: Feature,
    F::Geom: Geometry<Point = P>,
    S: Symbol<F>,
{
    pub fn new(features: Vec<F>, style: S, crs: Crs) -> Self {
        Self {
            features,
            symbol: style,
            crs,
            messenger: RwLock::new(None),
            lods: vec![Lod {
                min_resolution: 1.0,
                render_bundles: RwLock::new(vec![]),
                packed_bundles: RwLock::new(vec![]),
                feature_render_map: RwLock::new(Vec::new()),
            }],
            options: Default::default(),
            space: Default::default(),
        }
    }

    pub fn with_lods(features: Vec<F>, style: S, crs: Crs, lods: &[f64]) -> Self {
        let mut lods: Vec<_> = lods
            .iter()
            .map(|&min_resolution| Lod {
                min_resolution,
                render_bundles: RwLock::new(vec![]),
                packed_bundles: RwLock::new(vec![]),
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
            options: Default::default(),
            space: Default::default(),
        }
    }

    pub fn with_options(mut self, options: FeatureLayerOptions) -> Self {
        self.options = options;
        self
    }

    fn render_internal(&self, lod: &Lod, canvas: &mut dyn Canvas, view: &MapView) {
        let mut packed_bundles = lod.packed_bundles.write().unwrap();
        let mut bundles = lod.render_bundles.write().unwrap();
        for (index, bundle) in bundles.iter_mut().enumerate() {
            if packed_bundles.len() == index {
                packed_bundles.push(None);
            }

            if self.options.sort_by_depth {
                bundle.sort_by_depth(view);
            }

            if packed_bundles[index].is_none() || self.options.sort_by_depth {
                packed_bundles[index] = Some(canvas.pack_bundle(bundle))
            }
        }

        canvas.draw_bundles(
            &packed_bundles
                .iter()
                .filter_map(|v| v.as_ref().map(|v| &**v))
                .collect::<Vec<_>>(),
            RenderOptions {
                antialias: self.symbol.use_antialiasing(),
            },
        );
    }
}

impl<P, F, S> FeatureLayer<P, F, S, GeoSpace2d>
where
    P: NewGeoPoint + 'static,
    F: Feature,
    F::Geom: Geometry<Point = P>,
{
    pub fn extent_projected(&self, crs: &Crs) -> Option<Rect> {
        let projection = crs.get_projection::<P, Point2d>()?;
        self.features
            .iter()
            .filter_map(|f| f.geometry().project(&*projection))
            .map(|g| g.bounding_rectangle())
            .collect()
    }
}

impl<P, F, S> FeatureLayer<P, F, S, CartesianSpace2d>
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
    fn update_features_internal<Proj: Projection<InPoint = P, OutPoint = Point3d> + ?Sized>(
        &mut self,
        indices: &[usize],
        projection: impl Deref<Target = Proj>,
        in_place: bool,
    ) {
        if indices.is_empty() {
            return;
        }

        for lod in &self.lods {
            let mut bundles = lod.render_bundles.write().unwrap();
            let mut packed_bundles = lod.packed_bundles.write().unwrap();

            let mut feature_render_map = lod.feature_render_map.write().unwrap();
            if feature_render_map.is_empty() {
                return;
            }

            for index in indices {
                let entry = &mut feature_render_map[*index];
                let Some(bundle) = bundles.get_mut(entry.bundle_index) else {
                    return;
                };

                if !in_place {
                    for id in &entry.primitive_ids {
                        if let Err(err) = bundle.remove(*id) {
                            log::error!("Failed to remove primitive from the bundle: {err:?}");
                        }
                    }
                }

                let feature = self.features.get(*index).unwrap();
                if let Some(geometry) = feature.geometry().project(&*projection) {
                    let primitives = self.symbol.render(feature, &geometry, lod.min_resolution);

                    if primitives.len() == entry.primitive_ids.len() && in_place {
                        for (primitive, id) in
                            primitives.into_iter().zip(entry.primitive_ids.iter())
                        {
                            if let Err(err) = bundle.update(*id, primitive) {
                                log::error!("failed to update a primitive: {err:?}");
                            }
                        }
                    } else {
                        let mut ids = vec![];
                        for primitive in primitives {
                            ids.push(bundle.add(primitive, lod.min_resolution));
                        }

                        entry.primitive_ids = ids;
                    }
                }

                if let Some(bundle) = packed_bundles.get_mut(entry.bundle_index) {
                    bundle.take();
                }
            }

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

    fn render_with_projection<Proj: Projection<InPoint = P, OutPoint = Point3d> + ?Sized>(
        &self,
        view: &MapView,
        canvas: &mut dyn Canvas,
        projection: impl Deref<Target = Proj>,
    ) {
        if self.features.is_empty() {
            return;
        }

        let lod = self.select_lod(view.resolution());
        if lod.render_bundles.read().unwrap().is_empty() {
            let mut render_bundles = lod.render_bundles.write().unwrap();

            let mut bundle = canvas.create_bundle();
            let mut render_map = lod.feature_render_map.write().unwrap();

            for feature in &self.features {
                let Some(projected): Option<Geom<Point3d>> =
                    feature.geometry().project(&*projection)
                else {
                    continue;
                };

                let primitives = self.symbol.render(feature, &projected, lod.min_resolution);
                let ids = primitives
                    .into_iter()
                    .map(|primitive| bundle.add(primitive, lod.min_resolution))
                    .collect();

                render_map.push(RenderMapEntry {
                    bundle_index: render_bundles.len(),
                    primitive_ids: ids,
                });

                if bundle.approx_buffer_size() > self.options.buffer_size_limit {
                    let full_bundle = std::mem::replace(&mut bundle, canvas.create_bundle());
                    render_bundles.push(full_bundle);
                }
            }

            render_bundles.push(bundle);
        }

        self.render_internal(lod, canvas, view);
    }
}

impl<P, F, S> FeatureLayer<P, F, S, GeoSpace2d>
where
    P: NewGeoPoint + 'static,
    F: Feature + MaybeSend + MaybeSync + 'static,
    F::Geom: Geometry<Point = P>,
    S: Symbol<F> + MaybeSend + MaybeSync + 'static,
{
    fn get_projection(&self, crs: &Crs) -> impl Projection<InPoint = P, OutPoint = Point3d> {
        ChainProjection::new(
            crs.get_projection::<P, Point2d>().unwrap(),
            Box::new(AddDimensionProjection::new(0.0)),
        )
    }

    pub fn update_features(&mut self, indices: &[usize], view: &MapView, in_place: bool) {
        self.update_features_internal(indices, &self.get_projection(view.crs()), in_place)
    }
}

impl<P, F, S> Layer for FeatureLayer<P, F, S, GeoSpace2d>
where
    P: NewGeoPoint + 'static,
    F: Feature + MaybeSend + MaybeSync + 'static,
    F::Geom: Geometry<Point = P>,
    S: Symbol<F> + MaybeSend + MaybeSync + 'static,
{
    fn render(&self, view: &MapView, canvas: &mut dyn Canvas) {
        let projection = self.get_projection(view.crs());
        self.render_with_projection(view, canvas, &projection);
    }

    fn prepare(&self, _view: &MapView) {
        // do nothing
    }

    fn set_messenger(&mut self, messenger: Box<dyn Messenger>) {
        *self.messenger.write().unwrap() = Some(messenger);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl<P, F, S> FeatureLayer<P, F, S, CartesianSpace2d>
where
    P: NewCartesianPoint2d + Clone + 'static,
    F: Feature + MaybeSend + MaybeSync + 'static,
    F::Geom: Geometry<Point = P>,
    S: Symbol<F> + MaybeSend + MaybeSync + 'static,
{
    fn get_projection(&self, crs: &Crs) -> Box<dyn Projection<InPoint = P, OutPoint = Point3d>> {
        if crs == &self.crs {
            Box::new(AddDimensionProjection::new(0.0))
        } else {
            let self_proj = self.crs.get_projection::<GeoPoint2d, P>().unwrap();
            let view_proj: Box<dyn Projection<InPoint = _, OutPoint = Point2d>> =
                crs.get_projection().unwrap();
            Box::new(ChainProjection::new(
                Box::new(ChainProjection::new(
                    Box::new(InvertedProjection::new(self_proj)),
                    view_proj,
                )),
                Box::new(AddDimensionProjection::new(0.0)),
            ))
        }
    }

    pub fn update_features(&mut self, indices: &[usize], view: &MapView, in_place: bool) {
        self.update_features_internal(indices, self.get_projection(view.crs()), in_place)
    }
}

impl<P, F, S> Layer for FeatureLayer<P, F, S, CartesianSpace2d>
where
    P: NewCartesianPoint2d + Clone + 'static,
    F: Feature + MaybeSend + MaybeSync + 'static,
    F::Geom: Geometry<Point = P>,
    S: Symbol<F> + MaybeSend + MaybeSync + 'static,
{
    fn render(&self, view: &MapView, canvas: &mut dyn Canvas) {
        let projection = self.get_projection(view.crs());
        self.render_with_projection(view, canvas, projection);
    }

    fn prepare(&self, _view: &MapView) {
        // do nothing
    }

    fn set_messenger(&mut self, messenger: Box<dyn Messenger>) {
        *self.messenger.write().unwrap() = Some(messenger);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl<P, F, S> FeatureLayer<P, F, S, CartesianSpace3d>
where
    P: NewCartesianPoint3d + 'static,
    P::Num: AsPrimitive<f32>,
    F: Feature + MaybeSend + MaybeSync + 'static,
    F::Geom: Geometry<Point = P>,
    S: Symbol<F> + MaybeSend + MaybeSync + 'static,
{
    fn get_projection(&self) -> IdentityProjection<P, Point3d, CartesianSpace3d> {
        IdentityProjection::new()
    }

    pub fn update_features(&mut self, indices: &[usize], _view: &MapView, in_place: bool) {
        self.update_features_internal(indices, &self.get_projection(), in_place)
    }
}

impl<P, F, S> Layer for FeatureLayer<P, F, S, CartesianSpace3d>
where
    P: NewCartesianPoint3d + 'static,
    P::Num: AsPrimitive<f32>,
    F: Feature + MaybeSend + MaybeSync + 'static,
    F::Geom: Geometry<Point = P>,
    S: Symbol<F> + MaybeSend + MaybeSync + 'static,
{
    fn render(&self, view: &MapView, canvas: &mut dyn Canvas) {
        if view.crs() != &self.crs {
            // not supported at the moment for 3d coordiantes
            return;
        }

        let projection = self.get_projection();
        self.render_with_projection(view, canvas, &projection);
    }

    fn prepare(&self, _view: &MapView) {
        // do nothing
    }

    fn set_messenger(&mut self, messenger: Box<dyn Messenger>) {
        *self.messenger.write().unwrap() = Some(messenger);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
