//! Rendering backends for a map.
//!
//! The backends use [`Canvas`] instances to render map layers to the render target (screen, image, etc.).
//!
//! At this point only [`WgpuRenderer`] is implemented.

use crate::Color;
use galileo_types::cartesian::Size;
use maybe_sync::{MaybeSend, MaybeSync};
use render_bundle::RenderBundle;
use std::any::Any;

#[cfg(feature = "wgpu")]
mod wgpu;
#[cfg(feature = "wgpu")]
pub use wgpu::WgpuRenderer;

pub mod point_paint;
pub mod render_bundle;

#[derive(Debug, Copy, Clone, PartialEq, Hash)]
pub struct PrimitiveId(usize);

pub(crate) trait Renderer: MaybeSend + MaybeSync {
    fn as_any(&self) -> &dyn Any;
}

pub trait Canvas {
    fn size(&self) -> Size;
    fn create_bundle(&self) -> RenderBundle;
    fn pack_bundle(&self, bundle: &RenderBundle) -> Box<dyn PackedBundle>;
    fn draw_bundles(&mut self, bundles: &[&dyn PackedBundle], options: RenderOptions);
}

pub trait PackedBundle: MaybeSend + MaybeSync {
    fn as_any(&self) -> &dyn Any;
}

pub struct EmptyBundle {}
impl PackedBundle for EmptyBundle {
    fn as_any(&self) -> &dyn Any {
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
