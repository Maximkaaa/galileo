//! Example showing how to integrate Galileo map into your egui application.

use eframe::CreationContext;
use galileo::layer::raster_tile_layer::RasterTileLayerBuilder;
use galileo::{Map, MapBuilder};
use galileo_egui::{EguiMap, EguiMapState};
use galileo_types::geo::impls::GeoPoint2d;
use galileo_types::geo::GeoPoint;

const STORAGE_KEY: &str = "galileo_egui_app_example";

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
struct AppStorage {
    position: GeoPoint2d,
    resolution: f64,
}

struct EguiMapApp {
    map: EguiMapState,
    position: GeoPoint2d,
    resolution: f64,
}

impl EguiMapApp {
    fn new(egui_map_state: EguiMapState, cc: &CreationContext<'_>) -> Self {
        // get initial position from map
        let initial_position = egui_map_state
            .map()
            .view()
            .position()
            .expect("invalid map position");
        // get initial resolution from map
        let initial_resolution = egui_map_state.map().view().resolution();

        // Try to get stored values or use initial values
        let AppStorage {
            position,
            resolution,
        } = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, STORAGE_KEY))
            .unwrap_or(AppStorage {
                position: initial_position,
                resolution: initial_resolution,
            });

        Self {
            map: egui_map_state,
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

    // Called by egui to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(
            storage,
            STORAGE_KEY,
            &AppStorage {
                position: self.position,
                resolution: self.resolution,
            },
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    run()
}

pub(crate) fn run() {
    let map = create_map();
    galileo_egui::InitBuilder::new(map)
        .with_app_builder(|egui_map_state, cc| Box::new(EguiMapApp::new(egui_map_state, cc)))
        .init()
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
