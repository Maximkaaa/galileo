//! Map builder functions specific to Web target.

use crate::control::{EventProcessor, MapController};
use crate::galileo_map::{GalileoMap, MapBuilder};
use crate::layer::data_provider::dummy::DummyCacheController;
use crate::layer::data_provider::UrlImageProvider;
use crate::layer::data_provider::UrlSource;
use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::vector_tile_layer::tile_provider::loader::WebVtLoader;
use crate::layer::vector_tile_layer::tile_provider::VectorTileProvider;
use crate::layer::{RasterTileLayer, VectorTileLayer};
use crate::platform::web::vt_processor::WebWorkerVtProcessor;
use crate::platform::web::web_workers::WebWorkerService;
use crate::platform::{PlatformService, PlatformServiceImpl};
use crate::tile_scheme::TileIndex;
use crate::winit::WinitInputHandler;
use crate::TileSchema;
use galileo_types::cartesian::Size;
use galileo_types::geo::impls::GeoPoint2d;
use std::sync::{Arc, RwLock};
use wasm_bindgen::prelude::wasm_bindgen;
use winit::event_loop::{ControlFlow, EventLoop};

impl MapBuilder {
    /// Creates a raster tile layer.
    pub fn create_raster_tile_layer(
        tile_source: impl UrlSource<TileIndex> + 'static,
        tile_scheme: TileSchema,
    ) -> RasterTileLayer<UrlImageProvider<TileIndex>> {
        let tile_provider = UrlImageProvider::new(tile_source);
        RasterTileLayer::new(tile_scheme, tile_provider, None)
    }

    /// Create a new vector tile layer.
    pub async fn create_vector_tile_layer(
        tile_source: impl UrlSource<TileIndex> + 'static,
        tile_schema: TileSchema,
        style: VectorTileStyle,
    ) -> VectorTileLayer<WebVtLoader<DummyCacheController>, WebWorkerVtProcessor> {
        let tile_provider = Self::create_vector_tile_provider(tile_source, tile_schema.clone());
        VectorTileLayer::from_url(tile_provider, style, tile_schema).await
    }

    /// Create a new vector tile provider.
    pub fn create_vector_tile_provider(
        tile_source: impl UrlSource<TileIndex> + 'static,
        tile_schema: TileSchema,
    ) -> VectorTileProvider<WebVtLoader<DummyCacheController>, WebWorkerVtProcessor> {
        let loader = WebVtLoader::new(
            PlatformServiceImpl::new(),
            DummyCacheController {},
            tile_source,
        );
        let ww_service = WebWorkerService::new(4);
        let processor = WebWorkerVtProcessor::new(tile_schema, ww_service);

        #[allow(clippy::arc_with_non_send_sync)]
        VectorTileProvider::new(Arc::new(loader), Arc::new(processor))
    }
}

#[wasm_bindgen]
impl MapBuilder {
    /// Creates a new map builder and intializes console logger.
    pub fn new() -> Self {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));

        log::debug!("Logger is initialized");

        Self {
            position: GeoPoint2d::default(),
            resolution: 156543.03392800014 / 16.0,
            view: None,
            layers: vec![],
            event_handlers: vec![],
            window: None,
            event_loop: None,
            size: None,
            dom_container: None,
        }
    }

    /// Builds the map and adds it to the given parent HTML element.
    pub async fn build_into(mut self, container: web_sys::HtmlElement) -> GalileoMap {
        let event_loop = self
            .event_loop
            .take()
            .unwrap_or_else(|| EventLoop::new().expect("Failed to create event loop."));

        event_loop.set_control_flow(ControlFlow::Wait);

        log::info!("Trying to get window");

        let backend = Arc::new(RwLock::new(None));

        let input_handler = WinitInputHandler::default();

        let mut event_processor = EventProcessor::default();
        for handler in self.event_handlers.drain(..) {
            event_processor.add_handler(handler);
        }
        event_processor.add_handler(MapController::default());

        let width = container.offset_width() as u32;
        let height = container.offset_height() as u32;
        let size = Size::new(width, height);

        GalileoMap {
            window: None,
            map: self.build_map(None),
            backend,
            event_processor,
            input_handler,
            event_loop: Some(event_loop),
            init_size: size,
            dom_container: Some(container),
        }
    }

    /// Adds a new raster tile layer to the layer list.
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

        let tile_provider = UrlImageProvider::new(tile_source_int);
        self.layers.push(Box::new(RasterTileLayer::new(
            TileSchema::web(18),
            tile_provider,
            None,
        )));

        self
    }

    /// Sets DOM element to add the map canvas into.
    pub fn with_container(mut self, container: web_sys::HtmlElement) -> Self {
        self.dom_container = Some(container);
        self
    }
}

pub(crate) async fn sleep(duration: i32) {
    let mut cb = |resolve: js_sys::Function, _reject: js_sys::Function| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, duration)
            .unwrap();
    };

    let p = js_sys::Promise::new(&mut cb);

    wasm_bindgen_futures::JsFuture::from(p).await.unwrap();
}
