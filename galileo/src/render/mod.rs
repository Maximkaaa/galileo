use crate::primitives::Color;
use galileo_types::cartesian::size::Size;
use maybe_sync::{MaybeSend, MaybeSync};
use render_bundle::RenderBundle;
use std::any::Any;

#[cfg(feature = "wgpu")]
pub mod wgpu;

pub mod render_bundle;

#[derive(Debug, Copy, Clone, PartialEq, Hash)]
pub struct PrimitiveId(usize);

impl PrimitiveId {
    const INVALID: PrimitiveId = PrimitiveId(usize::MAX);
}

pub trait Renderer: MaybeSend + MaybeSync {
    fn create_bundle(&self) -> RenderBundle;
    fn pack_bundle(&self, bundle: RenderBundle) -> Box<dyn PackedBundle>;

    fn as_any(&self) -> &dyn Any;
}

pub trait Canvas {
    fn size(&self) -> Size;
    fn create_bundle(&self) -> RenderBundle;
    fn pack_bundle(&self, bundle: RenderBundle) -> Box<dyn PackedBundle>;
    fn pack_unpacked(&self, bundle: Box<dyn UnpackedBundle>) -> Box<dyn PackedBundle>;
    fn draw_bundles(&mut self, bundles: &[&dyn PackedBundle]);
}

pub trait PackedBundle: MaybeSend + MaybeSync {
    fn as_any(&self) -> &dyn Any;
    fn unpack(self: Box<Self>) -> Box<dyn UnpackedBundle>;
}

pub trait UnpackedBundle {
    fn modify_line(&mut self, id: PrimitiveId, paint: LinePaint);
    fn modify_polygon(&mut self, id: PrimitiveId, paint: Paint);
    fn modify_image(&mut self, id: PrimitiveId, paint: ImagePaint);
    fn modify_point(&mut self, id: PrimitiveId, paint: PointPaint);
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

#[derive(Debug, Clone, Copy)]
pub struct Paint {
    pub color: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct PointPaint {
    pub color: Color,
    pub size: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct LinePaint {
    pub color: Color,
    pub width: f64,
    pub offset: f64,
    pub line_cap: LineCap,
}

#[derive(Debug, Clone, Copy)]
pub struct PolygonPaint {
    pub color: Color,
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
