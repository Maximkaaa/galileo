#[cfg(feature = "raster_tiles")]
#[path = "../../galileo/examples/raster_tiles.rs"]
mod raster_tiles;

#[cfg(feature = "feature_layers")]
#[path = "../../galileo/examples/feature_layers.rs"]
mod feature_layers;

#[cfg(feature = "egui_app")]
#[path = "../../galileo/examples/egui_app.rs"]
mod egui_app;

#[cfg(feature = "georust")]
#[path = "../../galileo/examples/georust.rs"]
mod georust;

#[cfg(feature = "highlight_features")]
#[path = "../../galileo/examples/highlight_features.rs"]
mod highlight_features;

#[cfg(feature = "lambert")]
#[path = "../../galileo/examples/lambert.rs"]
mod lambert;

#[cfg(feature = "many_points")]
#[path = "../../galileo/examples/many_points.rs"]
mod many_points;

#[cfg(feature = "vector_tiles")]
#[path = "../../galileo/examples/vector_tiles.rs"]
mod vector_tiles;

#[cfg(feature = "add_remove_features")]
#[path = "../../galileo/examples/add_remove_features.rs"]
mod add_remove_features;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn main() {
    console_error_panic_hook::set_once();

    #[cfg(feature = "raster_tiles")]
    raster_tiles::run();

    #[cfg(feature = "feature_layers")]
    feature_layers::run();

    #[cfg(feature = "egui_app")]
    egui_app::run();

    #[cfg(feature = "georust")]
    georust::run();

    #[cfg(feature = "highlight_features")]
    highlight_features::run();

    #[cfg(feature = "lambert")]
    lambert::run();

    #[cfg(feature = "many_points")]
    many_points::run();

    #[cfg(feature = "vector_tiles")]
    vector_tiles::run();

    #[cfg(feature = "add_remove_features")]
    add_remove_features::run();
}
