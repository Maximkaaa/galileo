//! This exmpales shows how to set a simple map with a single raster tile layer.

use galileo::tile_scheme::TileSchema;
use galileo::{Map, MapBuilder, MapView};
use galileo_types::latlon;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    run()
}

pub(crate) fn run() {
    galileo_egui::init(create_map(), []).expect("failed to initialize");
}

fn create_map() -> Map {
    let layer = MapBuilder::create_raster_tile_layer(
        |index| {
            format!(
                "https://tile.openstreetmap.org/{}/{}/{}.png",
                index.z, index.x, index.y
            )
        },
        TileSchema::web(18),
    );

    Map::new(
        MapView::new(
            &latlon!(37.566, 128.9784),
            layer
                .tile_schema()
                .lod_resolution(8)
                .expect("invalid tile schema"),
        ),
        vec![Box::new(layer)],
        None,
    )
}

