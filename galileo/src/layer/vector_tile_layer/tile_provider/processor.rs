//! Vector tile processor.

use std::sync::Arc;

use galileo_mvt::MvtTile;
use maybe_sync::{MaybeSend, MaybeSync};
use serde::{Deserialize, Serialize};

use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::vector_tile_layer::tile_provider::VtStyleId;
use crate::render::render_bundle::RenderBundle;
use crate::tile_schema::TileIndex;

/// Error while processing vector tile.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TileProcessingError {
    /// Style with the given style id is not registered.
    InvalidStyle,
    /// Failed to render the tile.
    Rendering,
    /// Something went wrong.
    Internal,
}

/// Processor of vector tiles that converts raw tiles into render bundles ready to be displayed on
/// the map.
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait VectorTileProcessor: MaybeSend + MaybeSync {
    /// Returns try if a style with the given id was registered.
    fn has_style(&self, style_id: VtStyleId) -> bool;
    /// Returns a style with the given id.
    fn get_style(&self, style_id: VtStyleId) -> Option<Arc<VectorTileStyle>>;
    /// Registers a vector tile style.
    fn add_style(&self, style_id: VtStyleId, style: VectorTileStyle);
    /// Removes the style from the list.
    fn drop_style(&self, style_id: VtStyleId);
    /// Convert the tile into render bundle using the given style.
    ///
    /// The style with the given id must first be registered in the processor using
    /// [`VectorTileProcessor::add_style()`] method.
    async fn process_tile(
        &self,
        tile: Arc<MvtTile>,
        index: TileIndex,
        style_id: VtStyleId,
        dpi_scale_factor: f32,
    ) -> Result<RenderBundle, TileProcessingError>;
}
