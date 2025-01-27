use galileo_mvt::MvtTile;

use crate::layer::vector_tile_layer::tile_provider::loader::{TileLoadError, VectorTileLoader};
use crate::tile_scheme::TileIndex;

pub struct TestTileLoader {}

#[async_trait::async_trait]
impl VectorTileLoader for TestTileLoader {
    async fn load(&self, _index: TileIndex) -> Result<MvtTile, TileLoadError> {
        todo!()
    }
}
