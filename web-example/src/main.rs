#[cfg(feature = "raster_tiles")]
#[path = "../../galileo/examples/raster_tiles.rs"]
mod example;

#[cfg(feature = "feature_layers")]
#[path = "../../galileo/examples/feature_layers.rs"]
mod example;

#[cfg(feature = "egui_app")]
#[path = "../../galileo/examples/egui_app.rs"]
mod example;

fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        example::run();
    }
}
