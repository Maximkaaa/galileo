use eframe::{AppCreator, CreationContext};
use galileo::control::UserEventHandler;
use galileo::Map;

use crate::EguiMapState;

struct MapApp {
    map: EguiMapState,
}

impl MapApp {
    fn new(
        map: Map,
        cc: &CreationContext<'_>,
        handlers: impl IntoIterator<Item = Box<dyn UserEventHandler>>,
    ) -> Self {
        let ctx = cc.egui_ctx.clone();
        let render_state = cc
            .wgpu_render_state
            .clone()
            .expect("failed to get wgpu context");
        Self {
            map: EguiMapState::new(map, ctx, render_state, handlers),
        }
    }
}

impl eframe::App for MapApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.map.render(ui);
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn init(
    map: Map,
    handlers: impl IntoIterator<Item = Box<dyn UserEventHandler>>,
) -> eframe::Result {
    init_with_app(Box::new(|cc| Ok(Box::new(MapApp::new(map, cc, handlers)))))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn init_with_app(app_creator: AppCreator<'_>) -> eframe::Result {
    use std::time::Duration;

    use tokio::runtime::Runtime;

    env_logger::init();

    let rt = Runtime::new().expect("Unable to create Runtime");
    let _enter = rt.enter();

    std::thread::spawn(move || {
        rt.block_on(async {
            loop {
                tokio::time::sleep(Duration::from_secs(3600)).await;
            }
        })
    });

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 1000.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };

    eframe::run_native("Galileo Dev Map", native_options, app_creator)
}

#[cfg(target_arch = "wasm32")]
pub fn init(
    map: Map,
    handlers: impl IntoIterator<Item = Box<dyn UserEventHandler>> + 'static,
) -> eframe::Result {
    init_with_app(Box::new(|cc| Ok(Box::new(MapApp::new(map, cc, handlers)))))
}

#[cfg(target_arch = "wasm32")]
pub fn init_with_app(app_creator: AppCreator<'static>) -> eframe::Result {
    use eframe::wasm_bindgen::JsCast as _;

    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(canvas, web_options, app_creator)
            .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });

    Ok(())
}
