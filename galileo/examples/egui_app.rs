//! Example showing how to integrate Galileo map into your egui application.

use eframe::CreationContext;
use galileo::layer::raster_tile_layer::RasterTileLayerBuilder;
use galileo::{Map, MapBuilder};
use galileo_egui::{EguiMap, EguiMapState};
use galileo_types::geo::impls::GeoPoint2d;
use galileo_types::geo::GeoPoint;

struct EguiMapApp {
    map: EguiMapState,
    position: GeoPoint2d,
    resolution: f64,
}

impl EguiMapApp {
    fn new(map: Map, cc: &CreationContext) -> Self {
        let position = map.view().position().expect("invalid map position");
        let resolution = map.view().resolution();

        Self {
            map: EguiMapState::new(
                map,
                cc.egui_ctx.clone(),
                cc.wgpu_render_state.clone().expect("no render state"),
                [],
            ),
            position,
            resolution,
        }
    }
}

impl eframe::App for EguiMapApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            EguiMap::new(&mut self.map)
                .with_position(&mut self.position)
                .with_resolution(&mut self.resolution)
                .show_ui(ui);

            egui::Window::new("Galileo map").show(ctx, |ui| {
                ui.label("Map center position:");
                ui.label(format!(
                    "Lat: {:.4} Lon: {:.4}",
                    self.position.lat(),
                    self.position.lon()
                ));

                ui.separator();
                ui.label("Map resolution:");
                ui.label(format!("{:6}", self.resolution));
            });
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    run()
}

pub(crate) fn run() {
    let map = create_map();
    galileo_egui::init_with_app(Box::new(|cc| Ok(Box::new(EguiMapApp::new(map, cc)))))
        .expect("failed to initialize");
}

fn create_map() -> Map {
    let layer = RasterTileLayerBuilder::new_osm()
        .with_file_cache_checked(".tile_cache")
        .build()
        .expect("failed to create layer");

    MapBuilder::default()
        .with_latlon(37.566, 128.9784)
        .with_z_level(8)
        .with_layer(layer)
        .build()
}
