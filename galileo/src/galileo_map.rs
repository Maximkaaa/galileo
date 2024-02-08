use crate::control::custom::{CustomEventHandler, EventHandler};
use crate::control::event_processor::EventProcessor;
use crate::control::map::MapController;
use crate::layer::data_provider::file_cache::FileCacheController;
use crate::layer::data_provider::url_image_provider::{UrlImageProvider, UrlSource};
use crate::layer::raster_tile::RasterTileLayer;
use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::vector_tile_layer::VectorTileLayer;
use crate::layer::Layer;
use crate::map::Map;
use crate::render::wgpu::WgpuRenderer;
use crate::render::Renderer;
use crate::tile_scheme::{TileIndex, TileSchema};
use crate::view::MapView;
use crate::winit::{WinitInputHandler, WinitMessenger};
use galileo_types::cartesian::size::Size;
use galileo_types::geo::impls::point::GeoPoint2d;
use std::sync::{Arc, RwLock};
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

#[cfg(not(target_arch = "wasm32"))]
use crate::layer::data_provider::url_data_provider::UrlDataProvider;

#[cfg(target_arch = "wasm32")]
use crate::layer::data_provider::EmptyCache;
#[cfg(target_arch = "wasm32")]
use js_sys::wasm_bindgen::prelude::wasm_bindgen;

#[cfg(not(target_arch = "wasm32"))]
pub type VectorTileProvider =
    crate::layer::vector_tile_layer::tile_provider::rayon_provider::RayonProvider<
        UrlDataProvider<
            TileIndex,
            crate::layer::vector_tile_layer::tile_provider::vt_processor::VtProcessor,
            FileCacheController,
        >,
    >;

#[cfg(target_arch = "wasm32")]
pub type VectorTileProvider =
    crate::layer::vector_tile_layer::tile_provider::web_worker_provider::WebWorkerVectorTileProvider;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct GalileoMap {
    window: Arc<Window>,
    map: Arc<RwLock<Map>>,
    backend: Arc<RwLock<WgpuRenderer>>,
    event_processor: EventProcessor,
    input_handler: WinitInputHandler,
    event_loop: EventLoop<()>,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl GalileoMap {
    pub fn run(self) {
        let Self {
            window,
            map,
            backend,
            mut event_processor,
            mut input_handler,
            event_loop,
        } = self;

        event_loop
            .run(move |event, target| {
                target.set_control_flow(ControlFlow::Wait);

                match event {
                    Event::Resumed => {
                        log::info!("Resume called");
                        let size = window.inner_size();
                        if size.width > 0 && size.height > 0 {
                            let backend = backend.clone();
                            let window = window.clone();
                            let map = map.clone();
                            crate::async_runtime::spawn(async move {
                                let renderer = WgpuRenderer::new_with_window(
                                    &window,
                                    Size::new(size.width, size.height),
                                )
                                .await
                                .expect("failed to init renderer");

                                *backend.write().expect("poisoned lock") = renderer;
                                map.write()
                                    .expect("poisoned lock")
                                    .set_size(Size::new(size.width as f64, size.height as f64));
                                window.request_redraw();
                            });
                        }
                    }
                    Event::Suspended => {
                        backend
                            .write()
                            .expect("poisoned lock")
                            .clear_render_target();
                    }
                    Event::WindowEvent { event, window_id } if window_id == window.id() => {
                        if !backend.read().expect("poisoned lock").initialized() {
                            return;
                        }

                        match event {
                            WindowEvent::CloseRequested => {
                                target.exit();
                            }
                            WindowEvent::Resized(size) => {
                                log::info!("Window resized to: {size:?}");
                                backend
                                    .write()
                                    .unwrap()
                                    .resize(Size::new(size.width, size.height));

                                let mut map = map.write().unwrap();
                                map.set_size(Size::new(size.width as f64, size.height as f64));
                            }
                            WindowEvent::RedrawRequested => {
                                let cast: Arc<RwLock<dyn Renderer>> = backend.clone();
                                let map = map.read().unwrap();
                                map.load_layers(&cast);
                                backend.read().unwrap().render(&map).unwrap();
                            }
                            other => {
                                // Phone emulator in browsers works funny with scaling, using this code fixes it.
                                // But my real phone works fine without it, so it's commented out for now, and probably
                                // should be deleted later, when we know that it's not needed on any devices.

                                // #[cfg(target_arch = "wasm32")]
                                // let scale = window.scale_factor();
                                //
                                // #[cfg(not(target_arch = "wasm32"))]
                                let scale = 1.0;

                                if let Some(raw_event) =
                                    input_handler.process_user_input(&other, scale)
                                {
                                    let mut map = map.write().unwrap();
                                    event_processor.handle(
                                        raw_event,
                                        &mut map,
                                        &(*backend.read().unwrap()),
                                    );
                                }
                            }
                        }
                    }
                    Event::AboutToWait => {
                        map.write().unwrap().animate();
                    }
                    _ => (),
                }
            })
            .unwrap();
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct MapBuilder {
    position: GeoPoint2d,
    resolution: f64,
    view: Option<MapView>,
    layers: Vec<Box<dyn Layer>>,
    event_handlers: Vec<CustomEventHandler>,
    window: Option<Window>,
    event_loop: Option<EventLoop<()>>,
}

impl Default for MapBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl MapBuilder {
    pub fn new() -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Info).expect("Couldn't init logger");
        }

        Self {
            position: GeoPoint2d::default(),
            resolution: 156543.03392800014 / 16.0,
            view: None,
            layers: vec![],
            event_handlers: vec![],
            window: None,
            event_loop: None,
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn with_raster_tiles(mut self, tile_source: js_sys::Function) -> Self {
        let tile_source_int = move |index: &TileIndex| {
            log::info!("{index:?}");
            let this = wasm_bindgen::JsValue::null();
            tile_source
                .call1(&this, &(*index).into())
                .unwrap()
                .as_string()
                .unwrap()
        };

        let tile_provider = UrlImageProvider::new(tile_source_int, None::<EmptyCache>);
        self.layers.push(Box::new(RasterTileLayer::new(
            TileSchema::web(18),
            tile_provider,
            None,
        )));

        self
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn build_into(mut self, container: web_sys::Element) -> GalileoMap {
        use winit::platform::web::WindowExtWebSys;

        let event_loop = self
            .event_loop
            .take()
            .unwrap_or_else(|| EventLoop::new().unwrap());
        let window = self.window.take().unwrap_or_else(|| {
            WindowBuilder::new()
                .with_inner_size(PhysicalSize {
                    width: 1024,
                    height: 1024,
                })
                .build(&event_loop)
                .unwrap()
        });

        let canvas = web_sys::Element::from(window.canvas().unwrap());
        container.append_child(&canvas).unwrap();

        let width = container.client_width() as u32;
        let height = container.client_height() as u32;
        log::info!("Requesting canvas size: {width} - {height}");

        let _ = window.request_inner_size(PhysicalSize { width, height });

        sleep(1).await;

        self.window = Some(window);
        self.event_loop = Some(event_loop);

        self.build().await
    }
}

impl MapBuilder {
    pub async fn build(mut self) -> GalileoMap {
        let event_loop = self
            .event_loop
            .take()
            .unwrap_or_else(|| EventLoop::new().unwrap());

        log::info!("Trying to get window");

        let window = self.window.take().unwrap_or_else(|| {
            WindowBuilder::new()
                .with_inner_size(PhysicalSize {
                    width: 1024,
                    height: 1024,
                })
                .build(&event_loop)
                .unwrap()
        });

        let window = Arc::new(window);
        let messenger = WinitMessenger::new(window.clone());

        log::info!("Window size: {:?}", window.inner_size());
        let backend = WgpuRenderer::new()
            .await
            .expect("failed to create renderer");

        let backend = Arc::new(RwLock::new(backend));

        let input_handler = WinitInputHandler::default();

        let mut event_processor = EventProcessor::default();
        for handler in self.event_handlers.drain(..) {
            event_processor.add_handler(handler);
        }
        event_processor.add_handler(MapController::default());

        GalileoMap {
            window,
            map: self.build_map(messenger),
            backend,
            event_processor,
            input_handler,
            event_loop,
        }
    }

    pub fn with_window(mut self, window: Window) -> Self {
        self.window = Some(window);
        self
    }

    pub fn with_event_loop(mut self, event_loop: EventLoop<()>) -> Self {
        self.event_loop = Some(event_loop);
        self
    }

    pub fn center(mut self, position: GeoPoint2d) -> Self {
        self.position = position;
        self
    }

    pub fn resolution(mut self, resolution: f64) -> Self {
        self.resolution = resolution;
        self
    }

    pub fn with_view(mut self, view: MapView) -> Self {
        self.view = Some(view);
        self
    }

    pub fn create_raster_tile_layer(
        tile_source: impl UrlSource<TileIndex> + 'static,
        tile_scheme: TileSchema,
    ) -> RasterTileLayer<UrlImageProvider<TileIndex, FileCacheController>> {
        #[cfg(not(target_os = "android"))]
        let cache_controller = Some(FileCacheController::new(".tile_cache"));

        #[cfg(target_os = "android")]
        let cache_controller = Some(FileCacheController::new(
            "/data/data/com.example.rastertilesandroid/.tile_cache",
        ));

        let tile_provider = UrlImageProvider::new(tile_source, cache_controller);
        RasterTileLayer::new(tile_scheme, tile_provider, None)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_raster_tiles(
        mut self,
        tile_source: impl UrlSource<TileIndex> + 'static,
        tile_scheme: TileSchema,
    ) -> Self {
        self.layers.push(Box::new(Self::create_raster_tile_layer(
            tile_source,
            tile_scheme,
        )));
        self
    }

    pub fn create_vector_tile_layer(
        tile_source: impl UrlSource<TileIndex> + 'static,
        tile_scheme: TileSchema,
        style: VectorTileStyle,
    ) -> VectorTileLayer<VectorTileProvider> {
        #[cfg(not(target_arch = "wasm32"))]
        let tile_provider = VectorTileProvider::new(
            None,
            tile_scheme.clone(),
            UrlDataProvider::new(
                tile_source,
                crate::layer::vector_tile_layer::tile_provider::vt_processor::VtProcessor {},
                FileCacheController::new(".tile_cache"),
            ),
        );

        #[cfg(target_arch = "wasm32")]
        let tile_provider = VectorTileProvider::new(4, None, tile_source, tile_scheme.clone());
        VectorTileLayer::from_url(tile_provider, style, tile_scheme)
    }

    pub fn with_vector_tiles(
        mut self,
        tile_source: impl UrlSource<TileIndex> + 'static,
        tile_scheme: TileSchema,
        style: VectorTileStyle,
    ) -> Self {
        self.layers.push(Box::new(Self::create_vector_tile_layer(
            tile_source,
            tile_scheme,
            style,
        )));
        self
    }

    pub fn with_layer(mut self, layer: impl Layer + 'static) -> Self {
        self.layers.push(Box::new(layer));
        self
    }

    pub fn with_event_handler(mut self, handler: impl EventHandler + 'static) -> Self {
        let mut event_handler = CustomEventHandler::default();
        event_handler.set_input_handler(handler);
        self.event_handlers.push(event_handler);
        self
    }

    fn build_map(mut self, messenger: WinitMessenger) -> Arc<RwLock<Map>> {
        for layer in self.layers.iter_mut() {
            layer.set_messenger(Box::new(messenger.clone()))
        }

        let view = self
            .view
            .unwrap_or_else(|| MapView::new(&self.position, self.resolution));

        let map = Map::new(view, self.layers, Some(messenger));

        Arc::new(RwLock::new(map))
    }
}

#[cfg(target_arch = "wasm32")]
async fn sleep(duration: i32) {
    let mut cb = |resolve: js_sys::Function, _reject: js_sys::Function| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, duration)
            .unwrap();
    };

    let p = js_sys::Promise::new(&mut cb);

    wasm_bindgen_futures::JsFuture::from(p).await.unwrap();
}
