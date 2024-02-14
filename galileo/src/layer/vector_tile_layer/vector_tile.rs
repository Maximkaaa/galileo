use crate::render::PackedBundle;
use galileo_mvt::MvtTile;

/// Decoded and packed vector tile.
pub struct VectorTile {
    /// Original MVT tile.
    pub mvt_tile: MvtTile,
    /// Packed render bundle to draw this tile.
    pub bundle: Box<dyn PackedBundle>,
}
