use galileo::MapBuilder;
use wasm_bindgen::prelude::wasm_bindgen;

#[path = "../../common.rs"]
mod common;

#[path = "../../../galileo/examples/many_points.rs"]
mod example;

#[wasm_bindgen]
pub async fn init() {
    let (container, size) = common::set_up().await;
    example::run(
        MapBuilder::new()
            .with_size(size.width(), size.height())
            .with_container(container),
    )
    .await;
}
