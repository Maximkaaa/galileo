use crate::error::GalileoError;
use crate::layer::data_provider::FileCacheController;
use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::vector_tile_layer::tile_provider::VtStyleId;
use crate::render::render_bundle::RenderBundle;
use galileo_mvt::MvtTile;
use std::sync::Arc;

pub type CacheController = FileCacheController;

pub async fn add_style(_id: VtStyleId, _style: &VectorTileStyle) {}

pub async fn drop_style(_id: VtStyleId) {}

pub async fn prepare_tile(
    mvt_tile: Arc<MvtTile>,
    style: Arc<VectorTileStyle>,
) -> Result<RenderBundle, GalileoError> {
    todo!()
}
