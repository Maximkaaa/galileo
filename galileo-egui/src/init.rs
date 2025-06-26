use eframe::AppCreator;
use galileo::control::UserEventHandler;
use galileo::Map;

use crate::EguiMapState;

struct MapApp {
    pub map: EguiMapState,
}

impl eframe::App for MapApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.map.render(ui);
        });
    }
}

type AppBuilder = Box<dyn FnOnce(EguiMapState) -> Box<dyn eframe::App>>;

pub struct InitBuilder {
    map: Map,
    handlers: Vec<Box<dyn UserEventHandler>>,
    #[cfg(not(target_arch = "wasm32"))]
    native_options: Option<eframe::NativeOptions>,
    #[cfg(target_arch = "wasm32")]
    web_options: Option<eframe::WebOptions>,
    app_builder: Option<AppBuilder>,
}

impl InitBuilder {
    pub fn new(map: Map) -> Self {
        Self {
            map,
            handlers: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            native_options: None,
            #[cfg(target_arch = "wasm32")]
            web_options: None,
            app_builder: None,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_native_options(mut self, options: eframe::NativeOptions) -> Self {
        self.native_options = Some(options);
        self
    }

    #[cfg(target_arch = "wasm32")]
    pub fn with_web_options(mut self, options: eframe::WebOptions) -> Self {
        self.web_options = Some(options);
        self
    }

    pub fn with_handlers(
        mut self,
        handlers: impl IntoIterator<Item = Box<dyn UserEventHandler>>,
    ) -> Self {
        self.handlers.extend(handlers);
        self
    }

    pub fn with_app_builder(
        mut self,
        app_builder: impl FnOnce(EguiMapState) -> Box<dyn eframe::App> + 'static,
    ) -> Self {
        self.app_builder = Some(Box::new(app_builder));
        self
    }

    pub fn init(self) -> eframe::Result {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.init_not_wasm()
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.init_wasm()
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn init_not_wasm(self) -> eframe::Result {
        use std::time::Duration;

        use tokio::runtime::Runtime;

        env_logger::init();

        let handlers = self.handlers;

        let rt = Runtime::new().expect("Unable to create Runtime");
        let _enter = rt.enter();

        std::thread::spawn(move || {
            rt.block_on(async {
                loop {
                    tokio::time::sleep(Duration::from_secs(3600)).await;
                }
            })
        });

        let native_options = self.native_options.unwrap_or_default();

        let app_creator: AppCreator<'static> = app_creator(self.map, handlers, self.app_builder);

        eframe::run_native("Galileo Dev Map", native_options, app_creator)
    }

    #[cfg(target_arch = "wasm32")]
    fn init_wasm(self) -> eframe::Result {
        use eframe::wasm_bindgen::JsCast as _;

        let handlers = self.handlers;

        // Redirect `log` message to `console.log` and friends:
        eframe::WebLogger::init(log::LevelFilter::Info).ok();

        let web_options = self.web_options.unwrap_or_default();

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

            let app_creator: AppCreator<'static> =
                app_creator(self.map, handlers, self.app_builder);

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
}

fn app_creator<'app>(
    map: Map,
    handlers: Vec<Box<dyn UserEventHandler>>,
    app_builder: Option<AppBuilder>,
) -> eframe::AppCreator<'app> {
    Box::new(move |cc: &eframe::CreationContext<'_>| {
        let ctx = cc.egui_ctx.clone();
        let render_state = cc
            .wgpu_render_state
            .clone()
            .expect("failed to get wgpu context");
        let egui_map_state = EguiMapState::new(map, ctx, render_state, handlers);
        let app = app_builder.unwrap_or_else(|| {
            Box::new(|egui_map_state: EguiMapState| {
                Box::new(MapApp {
                    map: egui_map_state,
                })
            })
        })(egui_map_state);
        Ok(app)
    })
}
