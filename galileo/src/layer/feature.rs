use crate::layer::Layer;
use crate::primitives::{Color, Contour, Point2d, Polygon};
use crate::render::wgpu::WgpuRenderer;
use crate::render::{
    Canvas, LineCap, LinePaint, PackedBundle, Paint, PointPaint, RenderBundle, Renderer,
    UnpackedBundle,
};
use crate::view::MapView;
use galileo_types::geo::crs::Crs;
use galileo_types::geometry::{CartesianGeometry, CartesianPointType, GeoPointType};
use galileo_types::geometry::{Geometry, Point};
use galileo_types::CartesianPoint2dFloat;
use maybe_sync::{MaybeSend, MaybeSync};
use nalgebra::Point3;
use num_traits::Float;
use std::any::Any;
use std::sync::{Arc, RwLock};

pub trait Feature {
    type Geometry: Geometry;
    fn geometry(&self) -> &Self::Geometry;
}

pub struct FeatureLayer<Feature, Symbol> {
    features: Vec<Feature>,
    style: Symbol,
    render_bundle: RwLock<Option<Box<dyn PackedBundle>>>,
    feature_render_map: RwLock<Vec<Vec<usize>>>,
    crs: Crs,
}

impl<Feature, Symbol> FeatureLayer<Feature, Symbol> {
    pub fn new(features: Vec<Feature>, style: Symbol, crs: Crs) -> Self {
        Self {
            features,
            style,
            render_bundle: RwLock::new(None),
            feature_render_map: RwLock::new(Vec::new()),
            crs,
        }
    }

    pub fn get_features_at<N: Float, P>(
        &self,
        point: &impl CartesianPoint2dFloat<N>,
        tolerance: N,
    ) -> Vec<(usize, &Feature)>
    where
        P: Point<Type = CartesianPointType, Num = N>,
        Feature: CartesianGeometry<Point = P>,
    {
        self.features
            .iter()
            .enumerate()
            .filter(|(_, f)| f.is_point_inside(point, tolerance))
            .collect()
    }

    pub fn get_features_at_mut<N: Float, P>(
        &mut self,
        point: &impl CartesianPoint2dFloat<N>,
        tolerance: N,
    ) -> Vec<(usize, &mut Feature)>
    where
        P: Point<Type = CartesianPointType, Num = N>,
        Feature: CartesianGeometry<Point = P>,
    {
        self.features
            .iter_mut()
            .enumerate()
            .filter(|(_, f)| f.is_point_inside(point, tolerance))
            .collect()
    }

    pub fn features(&self) -> impl Iterator + '_ {
        self.features.iter()
    }

    pub fn features_mut(&mut self) -> impl Iterator<Item = &'_ mut Feature> + '_ {
        self.features.iter_mut()
    }
}

impl<F, S> FeatureLayer<F, S>
where
    F: Feature,
    S: Symbol<F, Geometry = <F as Feature>::Geometry>,
{
    // todo: remove deps on wgpu
    pub fn update_features(&mut self, indices: &[usize], renderer: &WgpuRenderer) {
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

        *bundle_lock = Some(renderer.pack_bundle(unpacked));
    }
}

pub trait Symbol<Feature> {
    type Geometry;

    fn render(
        &self,
        feature: &Feature,
        geometry: &Self::Geometry,
        bundle: &mut Box<dyn RenderBundle>,
    ) -> Vec<usize>;
    fn update(
        &self,
        feature: &Feature,
        renders_ids: &[usize],
        bundle: &mut Box<dyn UnpackedBundle>,
    );
}

impl<P, SP, G, SG, F, S> Layer for FeatureLayer<F, S>
where
    P: Point<Type = GeoPointType>,
    SP: Point<Type = CartesianPointType>,
    G: Geometry<Point = P>,
    SG: Geometry<Point = SP>,
    F: Feature<Geometry = G>,
    S: Symbol<F, Geometry = SG>,
{
    fn render<'a>(&self, position: &MapView, canvas: &'a mut dyn Canvas) {
        todo!()
    }

    fn prepare(&self, view: &MapView, renderer: &Arc<RwLock<dyn Renderer>>) {
        todo!()
    }

    fn as_any(&self) -> &dyn Any {
        todo!()
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        todo!()
    }
}

impl<P, G, F, S> Layer for FeatureLayer<F, S>
where
    P: Point<Type = CartesianPointType>,
    G: Geometry<Point = P>,
    F: Feature<Geometry = G> + MaybeSend + MaybeSync,
    S: Symbol<F, Geometry = <F as Feature>::Geometry> + MaybeSend + MaybeSync,
{
    fn render<'a>(&self, view: &MapView, canvas: &'a mut dyn Canvas) {
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

        canvas.draw_bundles(&[self.render_bundle.read().unwrap().as_ref().unwrap()]);
    }

    fn prepare(&self, view: &MapView, renderer: &Arc<RwLock<dyn Renderer>>) {
        // do nothing
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

impl Symbol<()> for CirclePointSymbol {
    type Geometry = Vec<Point3<f64>>;

    fn render(
        &self,
        _feature: &(),
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

    fn update(&self, _feature: &(), _renders_ids: &[usize], _bundle: &mut Box<dyn UnpackedBundle>) {
        todo!()
    }
}

pub struct SimpleLineSymbol {
    pub color: Color,
    pub width: f64,
}

impl Symbol<()> for SimpleLineSymbol {
    type Geometry = Contour<Point2d>;

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

impl Symbol<()> for SimplePolygonSymbol {
    type Geometry = Polygon<Point2d>;

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
