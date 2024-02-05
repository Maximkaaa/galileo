#[path = "../../common.rs"]
mod common;

use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub async fn init() {
    let (window, event_loop) = common::set_up().await;
    with_egui::run(window, event_loop).await;
}
