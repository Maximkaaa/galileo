use std::sync::Arc;

use galileo_types::cartesian::Size;
use galileo_types::geo::impls::GeoPoint2d;
use maybe_sync::{MaybeSend, MaybeSync};
use parking_lot::RwLock;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::window::Window;

use crate::control::{EventProcessor, EventPropagation, UserEvent};
use crate::layer::data_provider::UrlSource;
use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::Layer;
use crate::map::Map;
#[cfg(target_arch = "wasm32")]
use crate::platform::web::map_builder::sleep;
use crate::render::WgpuRenderer;
use crate::tile_scheme::{TileIndex, TileSchema};
use crate::view::MapView;
use crate::winit::{WinitInputHandler, WinitMessenger};
use crate::Messenger;

/// Convenience struct holding all necessary parts of a interactive map, including window handle and an event loop.
///
/// Usually an application using `Galileo` will have control over the window, event loop and rendering backend. This
/// structure can be used for developing map-related functionality separately from an application, or as a reference
/// of how to set up the event loop for Galileo map.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct GalileoMap {
    pub(crate) window: Option<Arc<Window>>,
    pub(crate) map: Arc<RwLock<Map>>,
    pub(crate) backend: Arc<RwLock<Option<WgpuRenderer>>>,
    pub(crate) event_processor: EventProcessor,
    pub(crate) input_handler: WinitInputHandler,
    pub(crate) event_loop: Option<EventLoop<()>>,
    pub(crate) init_size: Size<u32>,

    #[cfg(target_arch = "wasm32")]
    pub(crate) dom_container: Option<web_sys::HtmlElement>,
}

impl ApplicationHandler for GalileoMap {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_size = self.init_size;

        let window_attributes = Window::default_attributes().with_inner_size(PhysicalSize {
            width: window_size.width(),
            height: window_size.height(),
        });

        #[cfg(target_arch = "wasm32")]
        let window_attributes = {
            use wasm_bindgen::JsCast;
            use web_sys::HtmlCanvasElement;
            use winit::platform::web::WindowAttributesExtWebSys;

            let document = web_sys::window()
                .expect("failed to get window")
                .document()
                .expect("failed to get document");
            let canvas: HtmlCanvasElement = document
                .create_element("canvas")
                .expect("failed to create canvas")
                .unchecked_into();

            canvas.set_width(window_size.width());
            canvas.set_height(window_size.height());

            if let Some(dom_container) = &self.dom_container {
                dom_container
                    .append_child(&canvas.clone().unchecked_into())
                    .expect("failed to append canvas to the container");
            }

            window_attributes.with_canvas(Some(canvas))
        };

        let window = event_loop
            .create_window(window_attributes)
            .expect("Failed to init a window.");

        let window = Arc::new(window);

        self.window = Some(window.clone());
        let messenger = WinitMessenger::new(window.clone());

        self.set_messenger(Some(messenger));

        let backend = self.backend.clone();
        let map = self.map.clone();
        crate::async_runtime::spawn(async move {
            #[cfg(target_arch = "wasm32")]
            sleep(1).await;

            let size = window.inner_size();

            let mut renderer =
                WgpuRenderer::new_with_window(window.clone(), Size::new(size.width, size.height))
                    .await
                    .expect("failed to init renderer");

            let new_size = window.inner_size();
            if new_size != size {
                renderer.resize(Size::new(new_size.width, new_size.height));
            }

            *backend.write() = Some(renderer);
            map.write()
                .set_size(Size::new(size.width as f64, size.height as f64));
            window.request_redraw();
        });
    }

    fn suspended(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        *self.backend.write() = None;
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.map.write().animate();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                log::info!("Window resized to: {size:?}");
                if let Some(backend) = self.backend.write().as_mut() {
                    backend.resize(Size::new(size.width, size.height));

                    let mut map = self.map.write();
                    map.set_size(Size::new(size.width as f64, size.height as f64));
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(backend) = self.backend.read().as_ref() {
                    let map = self.map.read();
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

                if let Some(raw_event) = self.input_handler.process_user_input(&other, scale) {
                    let mut map = self.map.write();
                    self.event_processor.handle(raw_event, &mut map);
                }
            }
        }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl GalileoMap {
    fn set_messenger(&mut self, messenger: Option<WinitMessenger>) {
        let mut map = self.map.write();
        map.set_messenger(messenger.clone());

        if let Some(messenger) = messenger {
            for layer in map.layers_mut().iter_mut() {
                let boxed: Box<dyn Messenger> = Box::new(messenger.clone());
                layer.set_messenger(boxed);
            }
        }
    }

    /// Runs the main event loop.
    pub fn run(&mut self) {
        let event_loop = self.event_loop.take().expect("event loop is not created");
        event_loop.run_app(self).expect("failed to run application");
    }
}

#[cfg(target_arch = "wasm32")]
type EventHandler = dyn (Fn(&UserEvent, &mut Map) -> EventPropagation);
#[cfg(not(target_arch = "wasm32"))]
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
    pub(crate) size: Option<Size<u32>>,

    #[cfg(target_arch = "wasm32")]
    pub(crate) dom_container: Option<web_sys::HtmlElement>,
}

impl Default for MapBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MapBuilder {
    /// Construct [`GalileoMap`].
    pub async fn build(mut self) -> GalileoMap {
        let event_loop = self
            .event_loop
            .take()
            .unwrap_or_else(|| EventLoop::new().expect("Failed to create event loop."));

        event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

        log::info!("Trying to get window");

        let backend = Arc::new(RwLock::new(None));

        let input_handler = WinitInputHandler::default();

        let mut event_processor = EventProcessor::default();
        for handler in self.event_handlers.drain(..) {
            event_processor.add_handler(handler);
        }
        event_processor.add_handler(crate::control::MapController::default());
        let init_size = self.size.unwrap_or_else(|| Size::new(1024, 1024));

        #[cfg(target_arch = "wasm32")]
        let dom_container = self.dom_container.clone();

        GalileoMap {
            window: None,
            map: self.build_map(None),
            backend,
            event_processor,
            input_handler,
            event_loop: Some(event_loop),
            init_size,

            #[cfg(target_arch = "wasm32")]
            dom_container,
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

    /// Set the initial size of the map in pixels
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.size = Some(Size::new(width, height));
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

    pub(crate) fn build_map(mut self, messenger: Option<WinitMessenger>) -> Arc<RwLock<Map>> {
        if let Some(ref messenger) = messenger {
            for layer in self.layers.iter_mut() {
                layer.set_messenger(Box::new(messenger.clone()))
            }
        }

        let view = self
            .view
            .unwrap_or_else(|| MapView::new(&self.position, self.resolution));

        let map = Map::new(view, self.layers, messenger);

        Arc::new(RwLock::new(map))
    }
}
