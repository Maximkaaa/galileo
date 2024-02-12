use crate::control::custom::{CustomEventHandler, EventHandler};
use crate::control::event_processor::EventProcessor;
use crate::control::map::MapController;
use crate::layer::data_provider::UrlSource;
use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::Layer;
use crate::map::Map;
use crate::render::wgpu::WgpuRenderer;
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

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct GalileoMap {
    window: Arc<Window>,
    map: Arc<RwLock<Map>>,
    backend: Arc<RwLock<Option<WgpuRenderer>>>,
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
                        let backend = backend.clone();
                        let window = window.clone();
                        let map = map.clone();
                        crate::async_runtime::spawn(async move {
                            let size = window.inner_size();

                            let mut renderer = WgpuRenderer::new_with_window(
                                &window,
                                Size::new(size.width, size.height),
                            )
                            .await
                            .expect("failed to init renderer");

                            let new_size = window.inner_size();
                            if new_size != size {
                                renderer.resize(Size::new(new_size.width, new_size.height));
                            }

                            *backend.write().expect("poisoned lock") = Some(renderer);
                            map.write()
                                .expect("poisoned lock")
                                .set_size(Size::new(size.width as f64, size.height as f64));
                            window.request_redraw();
                        });
                    }
                    Event::Suspended => {
                        *backend.write().expect("poisoned lock") = None;
                    }
                    Event::WindowEvent { event, window_id } if window_id == window.id() => {
                        match event {
                            WindowEvent::CloseRequested => {
                                target.exit();
                            }
                            WindowEvent::Resized(size) => {
                                log::info!("Window resized to: {size:?}");
                                if let Some(backend) =
                                    backend.write().expect("lock is poisoned").as_mut()
                                {
                                    backend.resize(Size::new(size.width, size.height));

                                    let mut map = map.write().expect("lock is poisoned");
                                    map.set_size(Size::new(size.width as f64, size.height as f64));
                                }
                            }
                            WindowEvent::RedrawRequested => {
                                if let Some(backend) =
                                    backend.read().expect("lock is poisoned").as_ref()
                                {
                                    let map = map.read().expect("lock is poisoned");
                                    map.load_layers();
                                    if let Err(err) = backend.render(&map) {
                                        log::error!("Render error: {err:?}");
                                    }
                                }
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
                                    if let Some(backend) =
                                        backend.read().expect("lock is poisoned").as_ref()
                                    {
                                        let mut map = map.write().expect("lock is poisoned");
                                        event_processor.handle(raw_event, &mut map, backend);
                                    }
                                }
                            }
                        }
                    }
                    Event::AboutToWait => {
                        map.write().expect("lock is poisoned").animate();
                    }
                    _ => (),
                }
            })
            .expect("error processing event loop");
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct MapBuilder {
    pub(crate) position: GeoPoint2d,
    pub(crate) resolution: f64,
    pub(crate) view: Option<MapView>,
    pub(crate) layers: Vec<Box<dyn Layer>>,
    pub(crate) event_handlers: Vec<CustomEventHandler>,
    pub(crate) window: Option<Window>,
    pub(crate) event_loop: Option<EventLoop<()>>,
}

impl Default for MapBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MapBuilder {
    pub async fn build(mut self) -> GalileoMap {
        let event_loop = self
            .event_loop
            .take()
            .unwrap_or_else(|| EventLoop::new().expect("Failed to create event loop."));

        log::info!("Trying to get window");

        let window = self.window.take().unwrap_or_else(|| {
            WindowBuilder::new()
                .with_inner_size(PhysicalSize {
                    width: 1024,
                    height: 1024,
                })
                .build(&event_loop)
                .expect("Failed to init a window.")
        });

        let window = Arc::new(window);
        let messenger = WinitMessenger::new(window.clone());
        let backend = Arc::new(RwLock::new(None));

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
