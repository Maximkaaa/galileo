//! [`RenderBundle`] is used to store primitives and prepare them for rendering with the rendering backend.

use crate::decoded_image::DecodedImage;
use crate::error::GalileoError;
use crate::render::point_paint::PointPaint;
use crate::render::render_bundle::tessellating::TessellatingRenderBundle;
use crate::render::{ImagePaint, LinePaint, PolygonPaint, PrimitiveId};
use crate::view::MapView;
use galileo_types::cartesian::{CartesianPoint3d, Point2d};
use galileo_types::contour::Contour;
use galileo_types::Polygon;
use num_traits::AsPrimitive;
use std::borrow::Cow;

pub(crate) mod tessellating;

/// Render bundle is used to store render primitives and prepare them to be rendered with the rendering backend.
#[derive(Debug, Clone)]
pub struct RenderBundle(pub(crate) RenderBundleType);

#[derive(Debug, Clone)]
pub(crate) enum RenderBundleType {
    Tessellating(TessellatingRenderBundle),
}

impl RenderBundle {
    /// Returns approximate amount of memory used by this bundle.
    pub fn approx_buffer_size(&self) -> usize {
        match &self.0 {
            RenderBundleType::Tessellating(inner) => inner.approx_buffer_size(),
        }
    }

    /// Sets the value for `approx_buffer_size`.
    ///
    /// This can be useful for better memory management when used buffers size cannot be calculated
    /// properly.
    ///
    /// Note, that consequent changes to the bundle will change the given value as if it was the
    /// calculated one.
    pub fn set_approx_buffer_size(&mut self, size: usize) {
        match &mut self.0 {
            RenderBundleType::Tessellating(inner) => inner.set_approx_buffer_size(size),
        }
    }

    /// Set the clip area for drawing. Only primitives inside the clipped area will be displayed after rendering.
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

    /// Adds an image to the bundle.
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

    /// Adds a primitive to the bundle and returns the id of the given primitive in the bundle. The returned id can
    /// then be used to update or remove the primitive.
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

    /// Removes the primitive from the bundle.
    pub fn remove(&mut self, primitive_id: PrimitiveId) -> Result<(), GalileoError> {
        match &mut self.0 {
            RenderBundleType::Tessellating(inner) => inner.remove(primitive_id),
        }
    }

    /// Updates the primitive in the bundle if possible.
    ///
    /// This method cannot change the geometry of the primitive, so it will not attempt to check if the geometry
    /// changed or not. It will check the type of the primitive though and return an error in case the type is changed.
    ///
    /// This method can change the way a primitive is displayed very quickly, without performing expensive rendering
    /// calculation, but its capabilities are very limited.
    ///
    /// If the geometry may change, remove a primitive and add a new one instead.
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

    /// Returns true if the bundle has not primitives added.
    pub fn is_empty(&self) -> bool {
        match &self.0 {
            RenderBundleType::Tessellating(inner) => inner.is_empty(),
        }
    }

    /// Changes the style of the image.
    pub fn modify_image(&mut self, id: PrimitiveId, paint: ImagePaint) -> Result<(), GalileoError> {
        match &mut self.0 {
            RenderBundleType::Tessellating(inner) => inner.modify_image(id, paint),
        }
    }

    /// Sorts screen referenced primitives by depth relative to the camera position of the given `view`.
    pub fn sort_by_depth(&mut self, view: &MapView) {
        match &mut self.0 {
            RenderBundleType::Tessellating(inner) => inner.sort_by_depth(view),
        }
    }
}

/// Rendering primitive.
pub enum RenderPrimitive<'a, N, P, C, Poly>
where
    N: AsPrimitive<f32>,
    P: CartesianPoint3d<Num = N> + Clone,
    C: Contour<Point = P> + Clone,
    Poly: Polygon + Clone,
    Poly::Contour: Contour<Point = P>,
{
    /// Point primitive
    Point(Cow<'a, P>, Cow<'a, PointPaint<'a>>),
    /// Contour (line) primitive
    Contour(Cow<'a, C>, LinePaint),
    /// Polygon primitive
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
    /// Creates a new point primitive.
    pub fn new_point(point: P, paint: PointPaint<'a>) -> Self {
        Self::Point(Cow::Owned(point), Cow::Owned(paint))
    }

    /// Creates a new point primitive with the reference of the point.
    pub fn new_point_ref(point: &'a P, paint: &'a PointPaint<'a>) -> Self {
        Self::Point(Cow::Borrowed(point), Cow::Borrowed(paint))
    }

    /// Creates a new contour primitive
    pub fn new_contour(contour: C, paint: LinePaint) -> Self {
        Self::Contour(Cow::Owned(contour), paint)
    }

    /// Creates a new contour primitive with the reference of the contour
    pub fn new_contour_ref(contour: &'a C, paint: LinePaint) -> Self {
        Self::Contour(Cow::Borrowed(contour), paint)
    }

    /// Creates a new polygon primitive
    pub fn new_polygon(polygon: Poly, paint: PolygonPaint) -> Self {
        Self::Polygon(Cow::Owned(polygon), paint)
    }

    /// Creates a new polygon primitive with the reference of the polygon
    pub fn new_polygon_ref(polygon: &'a Poly, paint: PolygonPaint) -> Self {
        Self::Polygon(Cow::Borrowed(polygon), paint)
    }
}
