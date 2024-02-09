use crate::error::GalileoError;
use crate::primitives::DecodedImage;
use crate::render::point_paint::PointPaint;
use crate::render::{ImagePaint, LinePaint, PolygonPaint, PrimitiveId};
use crate::view::MapView;
use galileo_types::cartesian::impls::point::Point2d;
use galileo_types::cartesian::traits::cartesian_point::CartesianPoint3d;
use galileo_types::contour::Contour;
use galileo_types::polygon::Polygon;
use num_traits::AsPrimitive;
use std::borrow::Cow;
use tessellating::TessellatingRenderBundle;

pub mod tessellating;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum RenderBundle {
    Tessellating(TessellatingRenderBundle),
}

impl RenderBundle {
    pub fn approx_buffer_size(&self) -> usize {
        match self {
            RenderBundle::Tessellating(inner) => inner.approx_buffer_size(),
        }
    }

    pub fn clip_area<N, P, Poly>(&mut self, polygon: &Poly)
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
        Poly: Polygon,
        Poly::Contour: Contour<Point = P>,
    {
        match self {
            RenderBundle::Tessellating(inner) => inner.clip_area(polygon),
        }
    }

    pub fn add_image(
        &mut self,
        image: DecodedImage,
        vertices: [Point2d; 4],
        paint: ImagePaint,
    ) -> PrimitiveId {
        match self {
            RenderBundle::Tessellating(inner) => inner.add_image(image, vertices, paint),
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
        match self {
            RenderBundle::Tessellating(inner) => inner.add(primitive, min_resolution),
        }
    }

    pub fn remove(&mut self, primitive_id: PrimitiveId) -> Result<(), GalileoError> {
        match self {
            RenderBundle::Tessellating(inner) => inner.remove(primitive_id),
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
        match self {
            RenderBundle::Tessellating(inner) => inner.update(primitive_id, primitive),
        }
    }

    pub fn add_point<N, P>(&mut self, point: &P, paint: PointPaint) -> PrimitiveId
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
    {
        match self {
            RenderBundle::Tessellating(inner) => inner.add_point(point, paint),
        }
    }

    pub fn add_line<N, P, C>(
        &mut self,
        line: &C,
        paint: LinePaint,
        min_resolution: f64,
    ) -> PrimitiveId
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
        C: Contour<Point = P>,
    {
        match self {
            RenderBundle::Tessellating(inner) => inner.add_line(line, paint, min_resolution),
        }
    }

    pub fn add_polygon<N, P, Poly>(
        &mut self,
        polygon: &Poly,
        paint: PolygonPaint,
        min_resolution: f64,
    ) -> PrimitiveId
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
        Poly: Polygon,
        Poly::Contour: Contour<Point = P>,
    {
        match self {
            RenderBundle::Tessellating(inner) => inner.add_polygon(polygon, paint, min_resolution),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            RenderBundle::Tessellating(inner) => inner.is_empty(),
        }
    }

    pub fn modify_image(&mut self, id: PrimitiveId, paint: ImagePaint) -> Result<(), GalileoError> {
        match self {
            RenderBundle::Tessellating(inner) => inner.modify_image(id, paint),
        }
    }

    pub fn sort_by_depth(&mut self, view: &MapView) {
        match self {
            RenderBundle::Tessellating(inner) => inner.sort_by_depth(view),
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
