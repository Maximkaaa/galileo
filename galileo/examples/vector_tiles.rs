//! This example shows how to create and work with vector tile layers.

use std::sync::Arc;

use egui::FontDefinitions;
use galileo::control::{EventPropagation, MouseButton, UserEvent, UserEventHandler};
use galileo::layer::VectorTileLayer;
use galileo::layer::data_provider::remove_parameters_modifier;
use galileo::layer::vector_tile_layer::VectorTileLayerBuilder;
use galileo::layer::vector_tile_layer::style::VectorTileStyle;
use galileo::render::text::RustybuzzRasterizer;
use galileo::render::text::text_service::TextService;
use galileo::tile_schema::{TileIndex, TileSchema, TileSchemaBuilder};
use galileo::{Map, MapBuilder};
use galileo_egui::{EguiMap, EguiMapState};
use parking_lot::RwLock;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    run()
}

struct App {
    map: EguiMapState,
    layer: Arc<RwLock<VectorTileLayer>>,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            EguiMap::new(&mut self.map).show_ui(ui);
        });

        egui::Window::new("Buttons")
            .title_bar(false)
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Default style").clicked() {
                        self.set_style(default_style());
                    }
                    if ui.button("Gray style").clicked() {
                        self.set_style(gray_style());
                    }
                });
            });
    }
}

impl App {
    fn new(egui_map_state: EguiMapState, layer: Arc<RwLock<VectorTileLayer>>) -> Self {
        let fonts = FontDefinitions::default();
        let provider = RustybuzzRasterizer::default();

        let text_service = TextService::initialize(provider);
        for font in fonts.font_data.values() {
            text_service.load_font(Arc::new(font.font.to_vec()));
        }

        Self {
            map: egui_map_state,
            layer,
        }
    }

    fn set_style(&mut self, style: VectorTileStyle) {
        let mut layer = self.layer.write();
        if style != *layer.style() {
            layer.update_style(style);
            self.map.request_redraw();
        }
    }
}

pub(crate) fn run() {
    let Some(api_key) = std::option_env!("VT_API_KEY") else {
        panic!("Set the MapTiler API key into VT_API_KEY library when building this example");
    };

    let style = default_style();
    let layer = VectorTileLayerBuilder::new_rest(move |&index: &TileIndex| {
        format!(
            "https://api.maptiler.com/tiles/v3-openmaptiles/{z}/{x}/{y}.pbf?key={api_key}",
            z = index.z,
            x = index.x,
            y = index.y
        )
    })
    .with_style(style)
    .with_tile_schema(tile_schema())
    .with_file_cache_modifier_checked(".tile_cache", Box::new(remove_parameters_modifier))
    .with_attribution(
        "© MapTiler© OpenStreetMap contributors".to_string(),
        "https://www.maptiler.com/copyright/".to_string(),
    )
    .build()
    .expect("failed to create layer");

    let layer = Arc::new(RwLock::new(layer));

    let layer_copy = layer.clone();
    let handler = move |ev: &UserEvent, map: &mut Map| match ev {
        UserEvent::Click(MouseButton::Left, mouse_event) => {
            let view = map.view().clone();
            if let Some(position) = map
                .view()
                .screen_to_map(mouse_event.screen_pointer_position)
            {
                let features = layer_copy.read().get_features_at(&position, &view);

                for (layer, feature) in features {
                    println!("{layer}, {:?}", feature.properties);
                }
            }

            EventPropagation::Stop
        }
        _ => EventPropagation::Propagate,
    };

    let map = MapBuilder::default().with_layer(layer.clone()).build();
    galileo_egui::InitBuilder::new(map)
        .with_handlers([Box::new(handler) as Box<dyn UserEventHandler>])
        .with_app_builder(|egui_map_state, _| Box::new(App::new(egui_map_state, layer)))
        .init()
        .expect("failed to initialize");
}

fn default_style() -> VectorTileStyle {
    serde_json::from_str(include_str!("data/vt_style.json")).expect("invalid style json")
}

fn gray_style() -> VectorTileStyle {
    let style_str = r##"
{
  "rules": [
    {
      "symbol": {
        "line": {
          "stroke_color": "#000000ff",
          "width": 0.5
        }
      }
    },
    {
      "symbol": {
        "polygon": {
          "fill_color": "#999999ff"
        }
      }
    }
  ],
  "background": "#ffffffff"
}"##;
    serde_json::from_str(style_str).expect("invalid style json")
}

fn tile_schema() -> TileSchema {
    TileSchemaBuilder::web_mercator(2..16)
        .rect_tile_size(1024)
        .build()
        .expect("invalid tile schema")
}
