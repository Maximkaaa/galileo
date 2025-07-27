//! Helpers to initialize a simple egui application with a map. See documentation for
//! [`InitBuilder`].

use eframe::AppCreator;
use galileo::control::UserEventHandler;
use galileo::render::HorizonOptions;
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

type AppBuilder =
    Box<dyn FnOnce(EguiMapState, &eframe::CreationContext<'_>) -> Box<dyn eframe::App>>;

/// Helper constructor of a map application.
///
/// This structure is meant to be used primary for development purposes or in simple examples. If
/// you need more customization to your `egui` setup, you should create [`EguiMap`] widget
/// manually.
///
/// # Example
///
/// ```no_run
/// use galileo::layer::raster_tile_layer::RasterTileLayerBuilder;
/// use galileo::{Map, MapBuilder};
///
/// galileo_egui::InitBuilder::new(create_map())
///     .init()
///     .expect("failed to initialize");
///
/// fn create_map() -> Map {
///     let raster_layer = RasterTileLayerBuilder::new_osm()
///         .with_file_cache_checked(".tile_cache")
///         .build()
///         .expect("failed to create layer");
///
///     MapBuilder::default()
///         .with_latlon(37.566, 128.9784)
///         .with_z_level(8)
///         .with_layer(raster_layer)
///         .build()
/// }
/// ```
pub struct InitBuilder {
    map: Map,
    handlers: Vec<Box<dyn UserEventHandler>>,
    #[cfg(not(target_arch = "wasm32"))]
    native_options: Option<eframe::NativeOptions>,
    #[cfg(target_arch = "wasm32")]
    web_options: Option<eframe::WebOptions>,
    app_builder: Option<AppBuilder>,
    logging: bool,
    options: EguiMapOptions,
    #[cfg(not(target_arch = "wasm32"))]
    app_name: Option<String>,
    #[cfg(target_arch = "wasm32")]
    canvas_id: Option<String>,
}

/// Options of the map
pub struct EguiMapOptions {
    pub(crate) horizon_options: Option<HorizonOptions>,
}

impl Default for EguiMapOptions {
    fn default() -> Self {
        Self {
            horizon_options: Some(HorizonOptions::default()),
        }
    }
}

impl InitBuilder {
    /// Creates a new instance of the builder with the given Galileo map.
    pub fn new(map: Map) -> Self {
        Self {
            map,
            handlers: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            native_options: None,
            #[cfg(target_arch = "wasm32")]
            web_options: None,
            app_builder: None,
            logging: true,
            options: Default::default(),
            #[cfg(not(target_arch = "wasm32"))]
            app_name: None,
            #[cfg(target_arch = "wasm32")]
            canvas_id: None,
        }
    }

    /// Sets the native EGUI options.
    ///
    /// If not set, default options are used.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_native_options(mut self, options: eframe::NativeOptions) -> Self {
        self.native_options = Some(options);
        self
    }

    /// Sets the web EGUI options.
    ///
    /// If not set, default options are used.
    #[cfg(target_arch = "wasm32")]
    pub fn with_web_options(mut self, options: eframe::WebOptions) -> Self {
        self.web_options = Some(options);
        self
    }

    /// Adds the event handlers to the map.
    pub fn with_handlers(
        mut self,
        handlers: impl IntoIterator<Item = Box<dyn UserEventHandler>>,
    ) -> Self {
        self.handlers.extend(handlers);
        self
    }

    /// Sets a custom app builder.
    pub fn with_app_builder(
        mut self,
        app_builder: impl FnOnce(EguiMapState, &eframe::CreationContext<'_>) -> Box<dyn eframe::App>
            + 'static,
    ) -> Self {
        self.app_builder = Some(Box::new(app_builder));
        self
    }

    /// If `false` is set, `InitBuilder` will not initialize the logger.
    ///
    /// If not set or set to `true`, `env_logger` will be configured for native platforms or
    /// `console_log` for web.
    pub fn with_logging(mut self, logging: bool) -> Self {
        self.logging = logging;
        self
    }

    /// Sets the horizon options of the map.
    pub fn with_horizon_options(mut self, options: Option<HorizonOptions>) -> Self {
        self.options.horizon_options = options;
        self
    }

    /// Sets the name of the application window.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_app_name(mut self, app_name: &str) -> Self {
        self.app_name = Some(app_name.to_owned());
        self
    }

    /// Sets the `id` property of the canvas that the application will be rendered to.
    #[cfg(target_arch = "wasm32")]
    pub fn with_canvas_id(mut self, canvas_id: &str) -> Self {
        self.canvas_id = Some(canvas_id.to_owned());
        self
    }

    /// Starts the application.
    ///
    /// This function will block until the application is exited.
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

        if self.logging {
            env_logger::init();
        }

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

        let app_creator: AppCreator<'static> =
            app_creator(self.map, handlers, self.app_builder, self.options);

        let app_name: &str = self.app_name.as_deref().unwrap_or("Galileo Map App");

        eframe::run_native(app_name, native_options, app_creator)
    }

    #[cfg(target_arch = "wasm32")]
    fn init_wasm(self) -> eframe::Result {
        use eframe::wasm_bindgen::JsCast as _;

        let handlers = self.handlers;

        if self.logging {
            // Redirect `log` message to `console.log` and friends:
            eframe::WebLogger::init(log::LevelFilter::Info).ok();
        }

        let web_options = self.web_options.unwrap_or_default();

        wasm_bindgen_futures::spawn_local(async {
            let document = web_sys::window()
                .expect("No window")
                .document()
                .expect("No document");

            let canvas_id = self.canvas_id.unwrap_or("the_canvas_id".to_owned());
            let canvas = document
                .get_element_by_id(&canvas_id)
                .expect("Failed to find canvas element by an id")
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .expect("element found by an id was not an HtmlCanvasElement");

            let app_creator: AppCreator<'static> =
                app_creator(self.map, handlers, self.app_builder, self.options);

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
    options: EguiMapOptions,
) -> eframe::AppCreator<'app> {
    Box::new(move |cc: &eframe::CreationContext<'_>| {
        let ctx = cc.egui_ctx.clone();
        let render_state = cc
            .wgpu_render_state
            .clone()
            .expect("failed to get wgpu context");
        let egui_map_state = EguiMapState::new(map, ctx, render_state, handlers, options);
        let app = app_builder.unwrap_or_else(|| {
            Box::new(
                |egui_map_state: EguiMapState, _: &eframe::CreationContext<'_>| {
                    Box::new(MapApp {
                        map: egui_map_state,
                    })
                },
            )
        })(egui_map_state, cc);
        Ok(app)
    })
}
