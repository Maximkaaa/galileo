//! This example shows how to switch tile layers at runtime.

use galileo::layer::raster_tile_layer::RasterTileLayerBuilder;
use galileo::layer::RasterTileLayer;
use galileo::tile_schema::TileIndex;
use galileo::MapBuilder;
use galileo_egui::{EguiMap, EguiMapState};

struct App {
    map: EguiMapState,
}

impl App {
    fn new(egui_map_state: EguiMapState) -> Self {
        Self {
            map: egui_map_state,
        }
    }

    fn switch_layer(&mut self, tile_id: &str) {
        let layers = self.map.map_mut().layers_mut();
        // because we know to have one layer and at index 0 only, it's save to remove it using that index
        layers.remove(0);
        // create a new layer
        let layer = build_layer(tile_id);
        // add that layer
        layers.push(layer);
        // re-render the map
        self.map.request_redraw();
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            EguiMap::new(&mut self.map).show_ui(ui);
        });

        egui::Window::new("Buttons")
            .title_bar(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("winter").clicked() {
                        self.switch_layer("winter-v2");
                    }
                    if ui.button("streets").clicked() {
                        self.switch_layer("streets-v2");
                    }
                });
            });
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    run()
}

pub(crate) fn run() {
    let layer = build_layer("streets-v2");

    let map = MapBuilder::default()
        .with_latlon(37.566, 128.9784)
        .with_z_level(8)
        .with_layer(layer)
        .build();

    galileo_egui::InitBuilder::new(map)
        .with_app_builder(|egui_map_state, _| Box::new(App::new(egui_map_state)))
        .init()
        .expect("failed to initialize");
}

fn build_layer(tile_id: &str) -> RasterTileLayer {
    let Some(api_key) = std::option_env!("VT_API_KEY") else {
        panic!("Set the MapTiler API key into VT_API_KEY library when building this example");
    };
    let tile_id = tile_id.to_owned();
    RasterTileLayerBuilder::new_rest(move |&index: &TileIndex| {
        format!(
            "https://api.maptiler.com/maps/{tile_id}/{z}/{x}/{y}.png?key={api_key}",
            z = index.z,
            x = index.x,
            y = index.y
        )
    })
    .with_file_cache_checked(".tile_cache")
    .build()
    .expect("failed to create layer")
}
