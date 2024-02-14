use crate::control::{EventProcessor, EventPropagation, MapController, UserEvent};
use crate::layer::data_provider::UrlSource;
use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::Layer;
use crate::map::Map;
use crate::render::WgpuRenderer;
use crate::tile_scheme::{TileIndex, TileSchema};
use crate::view::MapView;
use crate::winit::{WinitInputHandler, WinitMessenger};
use galileo_types::cartesian::Size;
use galileo_types::geo::impls::GeoPoint2d;
use maybe_sync::{MaybeSend, MaybeSync};
use std::sync::{Arc, RwLock};
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

/// Convenience struct holding all necessary parts of a interactive map, including window handle and an event loop.
///
/// Usually an application using `Galileo` will have control over the window, event loop and rendering backend. This
/// structure can be used for developing map-related functionality separately from an application, or as a reference
/// of how to set up the event loop for Galileo map.
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
    /// Runs the main event loop.
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
                                    let mut map = map.write().expect("lock is poisoned");
                                    event_processor.handle(raw_event, &mut map);
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

type EventHandler = dyn (Fn(&UserEvent, &mut Map) -> EventPropagation) + MaybeSend + MaybeSync;

/// Builder for a [`GalileoMap`].
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct MapBuilder {
    pub(crate) position: GeoPoint2d,
    pub(crate) resolution: f64,
    pub(crate) view: Option<MapView>,
    pub(crate) layers: Vec<Box<dyn Layer>>,
    pub(crate) event_handlers: Vec<Box<EventHandler>>,
    pub(crate) window: Option<Window>,
    pub(crate) event_loop: Option<EventLoop<()>>,
}

impl Default for MapBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MapBuilder {
    /// Consturct [`GalileoMap`].
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

    /// Use the given window instead of creating a default one.
    pub fn with_window(mut self, window: Window) -> Self {
        self.window = Some(window);
        self
    }

    /// Use the given event loop instead of creating a default one.
    pub fn with_event_loop(mut self, event_loop: EventLoop<()>) -> Self {
        self.event_loop = Some(event_loop);
        self
    }

    /// Set the center of the map.
    pub fn center(mut self, position: GeoPoint2d) -> Self {
        self.position = position;
        self
    }

    /// Set the resolution of the map. For explanation about resolution, see [`MapView::resolution`].
    pub fn resolution(mut self, resolution: f64) -> Self {
        self.resolution = resolution;
        self
    }

    /// Set the view of the map.
    pub fn with_view(mut self, view: MapView) -> Self {
        self.view = Some(view);
        self
    }

    /// Add a vector tile layer with the given parameters.
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

    /// Add a give layer to the map.
    pub fn with_layer(mut self, layer: impl Layer + 'static) -> Self {
        self.layers.push(Box::new(layer));
        self
    }

    /// Add an event handler.
    pub fn with_event_handler(
        mut self,
        handler: impl (Fn(&UserEvent, &mut Map) -> EventPropagation) + MaybeSend + MaybeSync + 'static,
    ) -> Self {
        self.event_handlers.push(Box::new(handler));
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
