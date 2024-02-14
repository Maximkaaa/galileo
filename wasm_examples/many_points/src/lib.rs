use galileo::MapBuilder;
use wasm_bindgen::prelude::wasm_bindgen;

#[path = "../../common.rs"]
mod common;

#[path = "../../../galileo/examples/many_points.rs"]
mod example;

#[wasm_bindgen]
pub async fn init() {
    let (window, event_loop) = common::set_up().await;
    example::run(
        MapBuilder::new()
            .with_window(window)
            .with_event_loop(event_loop),
    )
    .await;
}
