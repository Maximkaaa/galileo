use crate::decoded_image::DecodedImage;
use crate::error::GalileoError;
use crate::render::point_paint::PointPaint;
use crate::render::{ImagePaint, LinePaint, PolygonPaint, PrimitiveId};
use crate::view::MapView;
use galileo_types::cartesian::CartesianPoint3d;
use galileo_types::cartesian::Point2d;
use galileo_types::contour::Contour;
use galileo_types::Polygon;
use num_traits::AsPrimitive;
use std::borrow::Cow;
use tessellating::TessellatingRenderBundle;

pub mod tessellating;

#[derive(Debug, Clone)]
pub struct RenderBundle(pub(crate) RenderBundleType);

#[derive(Debug, Clone)]
pub(crate) enum RenderBundleType {
    Tessellating(TessellatingRenderBundle),
}

impl RenderBundle {
    pub fn approx_buffer_size(&self) -> usize {
        match &self.0 {
            RenderBundleType::Tessellating(inner) => inner.approx_buffer_size(),
        }
    }

    pub fn clip_area<N, P, Poly>(&mut self, polygon: &Poly)
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
        Poly: Polygon,
        Poly::Contour: Contour<Point = P>,
    {
        match &mut self.0 {
            RenderBundleType::Tessellating(inner) => inner.clip_area(polygon),
        }
    }

    pub fn add_image(
        &mut self,
        image: DecodedImage,
        vertices: [Point2d; 4],
        paint: ImagePaint,
    ) -> PrimitiveId {
        match &mut self.0 {
            RenderBundleType::Tessellating(inner) => inner.add_image(image, vertices, paint),
        }
    }

    pub fn add<N, P, C, Poly>(
        &mut self,
        primitive: RenderPrimitive<N, P, C, Poly>,
        min_resolution: f64,
    ) -> PrimitiveId
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N> + Clone,
        C: Contour<Point = P> + Clone,
        Poly: Polygon + Clone,
        Poly::Contour: Contour<Point = P>,
    {
        match &mut self.0 {
            RenderBundleType::Tessellating(inner) => inner.add(primitive, min_resolution),
        }
    }

    pub fn remove(&mut self, primitive_id: PrimitiveId) -> Result<(), GalileoError> {
        match &mut self.0 {
            RenderBundleType::Tessellating(inner) => inner.remove(primitive_id),
        }
    }

    pub fn update<N, P, C, Poly>(
        &mut self,
        primitive_id: PrimitiveId,
        primitive: RenderPrimitive<N, P, C, Poly>,
    ) -> Result<(), GalileoError>
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N> + Clone,
        C: Contour<Point = P> + Clone,
        Poly: Polygon + Clone,
        Poly::Contour: Contour<Point = P>,
    {
        match &mut self.0 {
            RenderBundleType::Tessellating(inner) => inner.update(primitive_id, primitive),
        }
    }

    pub fn is_empty(&self) -> bool {
        match &self.0 {
            RenderBundleType::Tessellating(inner) => inner.is_empty(),
        }
    }

    pub fn modify_image(&mut self, id: PrimitiveId, paint: ImagePaint) -> Result<(), GalileoError> {
        match &mut self.0 {
            RenderBundleType::Tessellating(inner) => inner.modify_image(id, paint),
        }
    }

    pub fn sort_by_depth(&mut self, view: &MapView) {
        match &mut self.0 {
            RenderBundleType::Tessellating(inner) => inner.sort_by_depth(view),
        }
    }
}

pub enum RenderPrimitive<'a, N, P, C, Poly>
where
    N: AsPrimitive<f32>,
    P: CartesianPoint3d<Num = N> + Clone,
    C: Contour<Point = P> + Clone,
    Poly: Polygon + Clone,
    Poly::Contour: Contour<Point = P>,
{
    Point(Cow<'a, P>, PointPaint<'a>),
    Contour(Cow<'a, C>, LinePaint),
    Polygon(Cow<'a, Poly>, PolygonPaint),
}

impl<'a, N, P, C, Poly> RenderPrimitive<'a, N, P, C, Poly>
where
    N: AsPrimitive<f32>,
    P: CartesianPoint3d<Num = N> + Clone,
    C: Contour<Point = P> + Clone,
    Poly: Polygon + Clone,
    Poly::Contour: Contour<Point = P>,
{
    pub fn new_point(point: P, paint: PointPaint<'a>) -> Self {
        Self::Point(Cow::Owned(point), paint)
    }

    pub fn new_point_ref(point: &'a P, paint: PointPaint<'a>) -> Self {
        Self::Point(Cow::Borrowed(point), paint)
    }

    pub fn new_contour(contour: C, paint: LinePaint) -> Self {
        Self::Contour(Cow::Owned(contour), paint)
    }

    pub fn new_contour_ref(contour: &'a C, paint: LinePaint) -> Self {
        Self::Contour(Cow::Borrowed(contour), paint)
    }

    pub fn new_polygon(polygon: Poly, paint: PolygonPaint) -> Self {
        Self::Polygon(Cow::Owned(polygon), paint)
    }

    pub fn new_polygon_ref(polygon: &'a Poly, paint: PolygonPaint) -> Self {
        Self::Polygon(Cow::Borrowed(polygon), paint)
    }
}
