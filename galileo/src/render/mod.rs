use crate::Color;
use galileo_types::cartesian::size::Size;
use maybe_sync::{MaybeSend, MaybeSync};
use point_paint::PointPaint;
use render_bundle::RenderBundle;
use std::any::Any;

#[cfg(feature = "wgpu")]
pub mod wgpu;

pub mod point_paint;
pub mod render_bundle;

#[derive(Debug, Copy, Clone, PartialEq, Hash)]
pub struct PrimitiveId(usize);

pub trait Renderer: MaybeSend + MaybeSync {
    fn create_bundle(&self, lods: &Option<Vec<f32>>) -> RenderBundle;
    fn pack_bundle(&self, bundle: RenderBundle) -> Box<dyn PackedBundle>;

    fn as_any(&self) -> &dyn Any;
}

pub trait Canvas {
    fn size(&self) -> Size;
    fn create_bundle(&self, lods: &Option<Vec<f32>>) -> RenderBundle;
    fn pack_bundle(&self, bundle: RenderBundle) -> Box<dyn PackedBundle>;
    fn pack_unpacked(&self, bundle: Box<dyn UnpackedBundle>) -> Box<dyn PackedBundle>;
    fn draw_bundles(
        &mut self,
        bundles: &[&dyn PackedBundle],
        resolution: f32,
        options: RenderOptions,
    );
}

pub trait PackedBundle: MaybeSend + MaybeSync {
    fn as_any(&self) -> &dyn Any;
    fn unpack(self: Box<Self>) -> Box<dyn UnpackedBundle>;
}

pub trait UnpackedBundle {
    fn modify_line(&mut self, id: PrimitiveId, paint: LinePaint);
    fn modify_polygon(&mut self, id: PrimitiveId, paint: PolygonPaint);
    fn modify_image(&mut self, id: PrimitiveId, paint: ImagePaint);
    fn modify_point(&mut self, id: PrimitiveId, paint: PointPaint);
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

pub struct EmptyBundle {}
impl PackedBundle for EmptyBundle {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn unpack(self: Box<Self>) -> Box<dyn UnpackedBundle> {
        self
    }
}

impl UnpackedBundle for EmptyBundle {
    fn modify_line(&mut self, _id: PrimitiveId, _paint: LinePaint) {}
    fn modify_polygon(&mut self, _id: PrimitiveId, _paint: PolygonPaint) {}
    fn modify_image(&mut self, _id: PrimitiveId, _paint: ImagePaint) {}
    fn modify_point(&mut self, _id: PrimitiveId, _paint: PointPaint) {}
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

#[derive(Debug, Copy, Clone)]
pub struct RenderOptions {
    pub antialias: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self { antialias: true }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PolygonPaint {
    pub color: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct PreparedImage {
    _image_id: PreparedImageId,
}

pub(crate) type PreparedImageId = u64;

#[derive(Debug, Clone, Copy)]
pub struct LinePaint {
    pub color: Color,
    pub width: f64,
    pub offset: f64,
    pub line_cap: LineCap,
}

#[derive(Debug, Clone, Copy)]
pub enum LineCap {
    Round,
    Butt,
}

impl From<LineCap> for lyon::path::LineCap {
    fn from(val: LineCap) -> Self {
        match val {
            LineCap::Round => lyon::lyon_tessellation::LineCap::Round,
            LineCap::Butt => lyon::lyon_tessellation::LineCap::Butt,
        }
    }
}

pub struct ImagePaint {
    pub opacity: u8,
}
