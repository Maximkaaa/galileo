//! Rendering backends for a map.
//!
//! The backends use [`Canvas`] instances to render map layers to the render target (screen, image, etc.).
//!
//! At this point only [`WgpuRenderer`] is implemented.

use crate::Color;
use galileo_types::cartesian::Size;
use maybe_sync::{MaybeSend, MaybeSync};
use render_bundle::RenderBundle;
use serde::{Deserialize, Serialize};
use std::any::Any;

#[cfg(feature = "wgpu")]
mod wgpu;
#[cfg(feature = "wgpu")]
pub use wgpu::WgpuRenderer;

pub mod point_paint;
pub mod render_bundle;
pub mod text;

/// Id of a rendering primitive
#[derive(Debug, Copy, Clone, PartialEq, Hash)]
pub struct PrimitiveId(usize);

/// Canvas that a layer can be rendered to.
///
/// As layers can contain a lot of data, canvases use two-step process for rendering.
/// 1. Layers create [`RenderBundle`]s to store the primitives they want to render. All expensive calculation like
///    tessellation are done when a rendering primitive is added to the bundle. So to prevent frame rate drops, this can
///    be done in background threads or worker processes.
/// 2. When a bundle is ready to be drawn, it must be packed with [`Canvas::pack_bundle`] method. This moves data to
///    GPU buffers. Packed bundles cannot be modified and must be recreated in case the source `RenderBundle` changes.
/// 3. [`PackedBundle`]s can then be rendered by calling [`Canvas::draw_bundles`] method.
///
/// A layer may choose to store `RenderBundles` and `PackedBundles` between redraws to skip the expensive preparation
/// process.
pub trait Canvas {
    /// Size of the drawing area.
    fn size(&self) -> Size;
    /// Creates a new render bundle.
    fn create_bundle(&self) -> RenderBundle;
    /// Packs a bundle to make it ready for be rendered with [`Canvas::draw_bundles`] method.
    fn pack_bundle(&self, bundle: &RenderBundle) -> Box<dyn PackedBundle>;
    /// Render the bundles.
    fn draw_bundles(&mut self, bundles: &[&dyn PackedBundle], options: RenderOptions);
}

/// Packed render bundle ready to be drawn.
pub trait PackedBundle: MaybeSend + MaybeSync {
    /// Used to convert from trait object into a specific type by the rendering backend.
    fn as_any(&self) -> &dyn Any;
}

/// Rendering options.
#[derive(Debug, Copy, Clone)]
pub struct RenderOptions {
    /// If set to true, the primitives will be drawn using antialiasing (multisampling).
    pub antialias: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self { antialias: true }
    }
}

/// Parameters to draw a polygon primitive with.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PolygonPaint {
    /// Fill color of the polygon.
    pub color: Color,
}

/// Parameter to draw a line primitive with.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LinePaint {
    /// Color of the line.
    pub color: Color,
    /// Width of the line in pixels.
    pub width: f64,
    /// Offset of the line in pixels. The line is offset to the right side if the positive value is given, and to the
    /// left otherwise.
    pub offset: f64,
    /// Type of the cap of the line.
    pub line_cap: LineCap,
}

/// Cap (end point) style of the line.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum LineCap {
    /// Half-circle cap.
    Round,
    /// Strait rectangular cap.
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

/// Parameter to render an image with.
pub struct ImagePaint {
    /// Opacity of the image. The value of 255 means fully opaque image.
    ///
    /// If an image contains non-opaque pixels, the resulting opacity of those pixels is the product of the pixel
    /// opacity and this value represented in percents.
    pub opacity: u8,
}
