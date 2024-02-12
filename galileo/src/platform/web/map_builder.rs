use crate::galileo_map::{GalileoMap, MapBuilder};
use crate::layer::data_provider::url_image_provider::UrlImageProvider;
use crate::layer::data_provider::UrlSource;
use crate::layer::vector_tile_layer::style::VectorTileStyle;
use crate::layer::vector_tile_layer::tile_provider::web_worker_provider::WebWorkerVectorTileProvider;
use crate::layer::{RasterTileLayer, VectorTileLayer};
use crate::tile_scheme::TileIndex;
use crate::TileSchema;
use galileo_types::geo::impls::point::GeoPoint2d;
use wasm_bindgen::prelude::wasm_bindgen;
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

impl MapBuilder {
    pub fn create_raster_tile_layer(
        tile_source: impl UrlSource<TileIndex> + 'static,
        tile_scheme: TileSchema,
    ) -> RasterTileLayer<UrlImageProvider<TileIndex>> {
        let tile_provider = UrlImageProvider::new(tile_source);
        RasterTileLayer::new(tile_scheme, tile_provider, None)
    }

    pub fn create_vector_tile_layer(
        tile_source: impl UrlSource<TileIndex> + 'static,
        tile_scheme: TileSchema,
        style: VectorTileStyle,
    ) -> VectorTileLayer<WebWorkerVectorTileProvider> {
        let tile_provider =
            WebWorkerVectorTileProvider::new(4, None, tile_source, tile_scheme.clone());
        VectorTileLayer::from_url(tile_provider, style, tile_scheme)
    }
}

#[wasm_bindgen]
impl MapBuilder {
    pub fn new() -> Self {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(log::Level::Info).expect("Couldn't init logger");

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
}

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
