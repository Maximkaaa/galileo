#[cfg(feature = "raster_tiles")]
#[path = "../../galileo/examples/raster_tiles.rs"]
mod example;

fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        example::run();
    }
}
