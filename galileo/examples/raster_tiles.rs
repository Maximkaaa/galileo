//! This exmpales shows how to set a simple map with a single raster tile layer.

use galileo::layer::raster_tile_layer::RasterTileLayerBuilder;
use galileo::{Map, MapBuilder};

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    run()
}

pub(crate) fn run() {
    galileo_egui::init(create_map(), []).expect("failed to initialize");
}

fn create_map() -> Map {
    let raster_layer = RasterTileLayerBuilder::new_osm()
        .with_file_cache_checked(".tile_cache")
        .build()
        .expect("failed to create layer");

    MapBuilder::default()
        .with_latlon(37.566, 128.9784)
        .with_z_level(8)
        .with_layer(raster_layer)
        .build()
}
