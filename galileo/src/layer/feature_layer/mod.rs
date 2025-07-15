//! [`FeatureLayer`] stores features in a [`FeatureStore`] and renders them with a [`Symbol`].

use std::any::Any;
use std::marker::PhantomData;
use std::ops::Deref;

use galileo_types::cartesian::{
    CartesianPoint2d, NewCartesianPoint2d, NewCartesianPoint3d, Point2, Point3, Rect,
};
use galileo_types::geo::impls::projection::{AddDimensionProjection, IdentityProjection};
use galileo_types::geo::impls::GeoPoint2d;
use galileo_types::geo::{ChainProjection, Crs, InvertedProjection, NewGeoPoint, Projection};
use galileo_types::geometry::{CartesianGeometry2d, Geometry};
use galileo_types::geometry_type::{CartesianSpace2d, CartesianSpace3d, GeoSpace2d};
use maybe_sync::{MaybeSend, MaybeSync};
use num_traits::AsPrimitive;
use parking_lot::{Mutex, RwLock};

use crate::layer::attribution::Attribution;
use crate::layer::Layer;
use crate::messenger::Messenger;
use crate::render::{Canvas, RenderOptions};
use crate::view::MapView;

mod feature;
mod feature_store;
pub mod symbol;

mod bundle_store;
use bundle_store::{BundleStore, UpdateType};
pub use feature::Feature;
use feature_store::VecFeatureStore;
pub use feature_store::{FeatureId, FeatureStore};
pub use symbol::Symbol;

/// Feature layers render a set of [features](Feature) using [symbols](Symbol).
///
/// After the layer is created, the [internal features storage](FeatureStore) can be accessed through [FeatureLayer::features] and
/// [FeatureLayer::features_mut] methods. This storage provides methods to edit features or hide/show them without
/// deleting from the layer.
///
/// All features added to the layer must be in the `CRS` of the layer. Layer will not attempt to convert geometries
/// from incorrect CRS (as there's no way for the layer to know which CRS the geometry is projected to). On the other
/// hand, the CRS of the layer doesn't have to be same as the CRS of the map. When the layer is requested to be rendered,
/// it will project all its features into needed CRS automatically.
///
/// Feature layer can render features differently at different resolutions. See [`FeatureLayer::with_lods`] for
/// details.
pub struct FeatureLayer<P, F, S, Space>
where
    F: Feature,
    F::Geom: Geometry<Point = P>,
{
    features: Box<dyn FeatureStore<F>>,
    symbol: S,
    crs: Crs,
    lods: Vec<Lod>,
    messenger: RwLock<Option<Box<dyn Messenger>>>,
    options: FeatureLayerOptions,

    space: PhantomData<Space>,
}

/// Configuration of a [FeatureLayer].
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

    /// If set to true, the layer will be rendered with anti-aliasing. It makes rendered lines look smoother but is a
    /// little less performant.
    pub use_antialiasing: bool,
}

impl Default for FeatureLayerOptions {
    fn default() -> Self {
        Self {
            sort_by_depth: false,
            buffer_size_limit: 10_000_000,
            use_antialiasing: true,
        }
    }
}

struct Lod {
    min_resolution: f64,
    bundles: Mutex<BundleStore>,
}

impl Lod {
    fn new(min_resolution: f64, bundle_size_limit: usize) -> Self {
        Self {
            min_resolution,
            bundles: Mutex::new(BundleStore::new(bundle_size_limit)),
        }
    }
}

impl<P, F, S, Space> FeatureLayer<P, F, S, Space>
where
    F: Feature + MaybeSend + MaybeSync + 'static,
    F::Geom: Geometry<Point = P>,
    S: Symbol<F>,
{
    /// Creates a new layer with the given parameters.
    pub fn new(features: Vec<F>, style: S, crs: Crs) -> Self {
        let options = FeatureLayerOptions::default();
        Self {
            features: Box::new(VecFeatureStore::new(features)),
            symbol: style,
            crs,
            messenger: RwLock::new(None),
            lods: vec![Lod::new(1.0, options.buffer_size_limit)],
            options,
            space: Default::default(),
        }
    }

    /// Creates a new layer with specified levels of detail.
    ///
    /// Levels of details specify resolution boundaries at which feature must be rendered separately.
    pub fn with_lods(features: Vec<F>, style: S, crs: Crs, lods: &[f64]) -> Self {
        let options = FeatureLayerOptions::default();
        let mut lods: Vec<_> = lods
            .iter()
            .map(|&min_resolution| Lod::new(min_resolution, options.buffer_size_limit))
            .collect();
        lods.sort_by(|a, b| b.min_resolution.total_cmp(&a.min_resolution));

        Self {
            features: Box::new(VecFeatureStore::new(features)),
            symbol: style,
            crs,
            messenger: RwLock::new(None),
            lods,
            options,
            space: Default::default(),
        }
    }

    /// Set the rendering options for the layer.
    pub fn with_options(mut self, options: FeatureLayerOptions) -> Self {
        self.options = options;

        for lod in &self.lods {
            let mut store = lod.bundles.lock();
            store.set_bundle_size_limit(options.buffer_size_limit);
        }

        self
    }

    /// Returns a reference to the feature store.
    pub fn features(&self) -> &dyn FeatureStore<F> {
        &*self.features
    }

    /// Returns a mutable reference to the feature store.
    pub fn features_mut(&mut self) -> &mut dyn FeatureStore<F> {
        &mut *self.features
    }

    /// Returns the CRS of the layer.
    pub fn crs(&self) -> &Crs {
        &self.crs
    }

    /// Changes the symbol of the layer.
    pub fn set_symbol(&mut self, symbol: S) {
        self.symbol = symbol;
        self.drop_render_cache();
        self.request_redraw();
    }

    /// Marks the feature with the given id to be redrawn on the next render cycles.
    pub fn update_feature(&self, feature_id: FeatureId) {
        for lod in &self.lods {
            lod.bundles.lock().reset_feature(feature_id);
        }
    }

    /// Rerenders all features in the layer.
    pub fn update_all_features(&mut self) {
        self.drop_render_cache();
    }

    fn drop_render_cache(&mut self) {
        for lod in &mut self.lods {
            let mut bundles = lod.bundles.lock();
            bundles.clear();
        }
    }

    fn request_redraw(&self) {
        if let Some(messenger) = self.messenger.read().as_ref() {
            messenger.request_redraw();
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

    fn render_with_projection<Proj: Projection<InPoint = P, OutPoint = Point3> + ?Sized>(
        &self,
        view: &MapView,
        canvas: &mut dyn Canvas,
        projection: impl Deref<Target = Proj>,
    ) {
        let lod = self.select_lod(view.resolution());
        let mut store = lod.bundles.lock();
        store.set_dpi_scale_factor(canvas.dpi_scale_factor());

        match store.required_update() {
            UpdateType::All => {
                for (id, feature) in self.features.iter() {
                    store.with_bundle(|bundle| {
                        if let Some(projected) = feature.geometry().project(&*projection) {
                            self.symbol
                                .render(feature, &projected, lod.min_resolution, bundle);
                        }

                        id
                    });
                }
            }
            UpdateType::Selected(ids) => {
                for id in ids {
                    let Some(feature) = self.features.get(id) else {
                        continue;
                    };
                    store.with_bundle(|bundle| {
                        if let Some(projected) = feature.geometry().project(&*projection) {
                            self.symbol
                                .render(feature, &projected, lod.min_resolution, bundle);
                        }

                        id
                    });
                }
            }
            UpdateType::None => {}
        }

        store.pack(canvas);

        canvas.draw_bundles(
            &store.packed(),
            RenderOptions {
                antialias: self.options.use_antialiasing,
            },
        );
    }
}

impl<P, F, S> FeatureLayer<P, F, S, GeoSpace2d>
where
    P: NewGeoPoint + 'static,
    F: Feature + MaybeSync + MaybeSend,
    F::Geom: Geometry<Point = P>,
{
    /// Extend (bounding rectangle) of the layer, projected into given CRS.
    ///
    /// If the layer doesn't contain any features, or if at least one of them cannot be projected into the given
    /// CRS, `None` will be returned.
    pub fn extent_projected(&self, crs: &Crs) -> Option<Rect> {
        let projection = crs.get_projection::<P, Point2>()?;
        self.features
            .iter()
            .filter_map(|(_, f)| f.geometry().project(&*projection))
            .filter_map(|g| g.bounding_rectangle())
            .collect()
    }
}

impl<P, F, S> FeatureLayer<P, F, S, CartesianSpace2d>
where
    P: CartesianPoint2d,
    F: Feature + MaybeSync + MaybeSend,
    F::Geom: Geometry<Point = P>,
{
    /// Returns an iterator of features that are within `tolerance` units from the `point`. Note that the `point` is
    /// expected to be set in the layer's CRS.
    ///
    /// At this moment this method just iterates over all features checking for each one if it is at the point. But
    /// in future it may be changed into using geo-index to make this more efficient. So this method should be preferred
    /// to manually checking every feature.
    pub fn get_features_at<'a>(
        &'a self,
        point: &'a impl CartesianPoint2d<Num = P::Num>,
        tolerance: P::Num,
    ) -> impl Iterator<Item = (FeatureId, &'a F)> + 'a
    where
        F::Geom: CartesianGeometry2d<P>,
    {
        self.features
            .iter()
            .filter(move |(_, f)| f.geometry().is_point_inside(point, tolerance))
    }

    /// Returns a mutable iterator of features that are within `tolerance` units from the `point`. Note that the `point` is
    /// expected to be set in the layer's CRS.
    ///
    /// At this moment this method just iterates over all features checking for each one if it is at the point. But
    /// in future it may be changed into using geo-index to make this more efficient. So this method should be preferred
    /// to manually checking every feature.
    pub fn get_features_at_mut<'a>(
        &'a mut self,
        point: &'a impl CartesianPoint2d<Num = P::Num>,
        tolerance: P::Num,
    ) -> impl Iterator<Item = (FeatureId, &'a mut F)> + 'a
    where
        F::Geom: CartesianGeometry2d<P>,
    {
        self.features
            .iter_mut()
            .filter(move |(_, f)| f.geometry().is_point_inside(point, tolerance))
    }
}

impl<P, F, S> FeatureLayer<P, F, S, GeoSpace2d>
where
    P: NewGeoPoint + 'static,
    F: Feature + MaybeSend + MaybeSync + 'static,
    F::Geom: Geometry<Point = P>,
    S: Symbol<F> + MaybeSend + MaybeSync + 'static,
{
    fn get_projection(&self, crs: &Crs) -> Option<impl Projection<InPoint = P, OutPoint = Point3>> {
        Some(ChainProjection::new(
            crs.get_projection::<P, Point2>()?,
            Box::new(AddDimensionProjection::new(0.0)),
        ))
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
        let Some(projection) = self.get_projection(view.crs()) else {
            return;
        };
        self.render_with_projection(view, canvas, &projection);
    }

    fn prepare(&self, _view: &MapView, _canvas: &mut dyn Canvas) {
        // do nothing
    }

    fn set_messenger(&mut self, messenger: Box<dyn Messenger>) {
        *self.messenger.write() = Some(messenger);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn attribution(&self) -> Option<Attribution> {
        None
    }
}

impl<P, F, S> FeatureLayer<P, F, S, CartesianSpace2d>
where
    P: NewCartesianPoint2d + Clone + 'static,
    F: Feature + MaybeSend + MaybeSync + 'static,
    F::Geom: Geometry<Point = P>,
    S: Symbol<F> + MaybeSend + MaybeSync + 'static,
{
    fn get_projection(
        &self,
        crs: &Crs,
    ) -> Option<Box<dyn Projection<InPoint = P, OutPoint = Point3>>> {
        if crs == &self.crs {
            Some(Box::new(AddDimensionProjection::new(0.0)))
        } else {
            let self_proj = self.crs.get_projection::<GeoPoint2d, P>()?;
            let view_proj: Box<dyn Projection<InPoint = _, OutPoint = Point2>> =
                crs.get_projection()?;
            Some(Box::new(ChainProjection::new(
                Box::new(ChainProjection::new(
                    Box::new(InvertedProjection::new(self_proj)),
                    view_proj,
                )),
                Box::new(AddDimensionProjection::new(0.0)),
            )))
        }
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
        let Some(projection) = self.get_projection(view.crs()) else {
            return;
        };
        self.render_with_projection(view, canvas, projection);
    }

    fn prepare(&self, _view: &MapView, _canvas: &mut dyn Canvas) {
        // do nothing
    }

    fn set_messenger(&mut self, messenger: Box<dyn Messenger>) {
        *self.messenger.write() = Some(messenger);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn attribution(&self) -> Option<Attribution> {
        None
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
    fn get_projection(&self) -> IdentityProjection<P, Point3, CartesianSpace3d> {
        IdentityProjection::new()
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
            // not supported at the moment for 3d coordinates
            return;
        }

        let projection = self.get_projection();
        self.render_with_projection(view, canvas, &projection);
    }

    fn prepare(&self, _view: &MapView, _canvas: &mut dyn Canvas) {
        // do nothing
    }

    fn set_messenger(&mut self, messenger: Box<dyn Messenger>) {
        *self.messenger.write() = Some(messenger);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn attribution(&self) -> Option<Attribution> {
        None
    }
}
