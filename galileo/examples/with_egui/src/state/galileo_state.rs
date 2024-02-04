use std::sync::{Arc, RwLock};

use galileo::{
    control::{event_processor::EventProcessor, map::MapController},
    layer::data_provider::file_cache::FileCacheController,
    render::wgpu::WgpuRenderer,
    tile_scheme::TileIndex,
    view::MapView,
    winit::WinitInputHandler,
    TileScheme,
};
use galileo_types::{cartesian::size::Size, latlon};
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};
use winit::{dpi::PhysicalSize, window::Window};

use super::WgpuFrame;

pub struct GalileoState {
    input_handler: WinitInputHandler,
    event_processor: EventProcessor,
    renderer: Arc<RwLock<WgpuRenderer>>,
    map: Arc<RwLock<galileo::map::Map>>,
}

impl GalileoState {
    pub fn new(
        window: Arc<Window>,
        device: Arc<Device>,
        surface: Arc<Surface>,
        queue: Arc<Queue>,
        config: SurfaceConfiguration,
        size: winit::dpi::PhysicalSize<u32>,
    ) -> Self {
        let messenger = galileo::winit::WinitMessenger::new(window);

        let renderer = WgpuRenderer::create_with_surface(
            device,
            surface,
            queue,
            config,
            Size::new(size.width, size.height),
        );
        let renderer = Arc::new(RwLock::new(renderer));

        let input_handler = WinitInputHandler::default();

        let mut event_processor = EventProcessor::default();
        event_processor.add_handler(MapController::default());

        let view = MapView::new(
            &latlon!(37.566, 126.9784),
            TileScheme::web(18).lod_resolution(8).unwrap(),
        );

        #[cfg(not(target_arch = "wasm32"))]
        let cache_controller = Some(FileCacheController::new(".tile_cache"));
        #[cfg(target_arch = "wasm32")]
        let cache_controller: Option<FileCacheController> = None;

        let tile_source = |index: &TileIndex| {
            format!(
                "https://tile.openstreetmap.org/{}/{}/{}.png",
                index.z, index.x, index.y
            )
        };

        let tile_provider =
            galileo::layer::data_provider::url_image_provider::UrlImageProvider::new(
                tile_source,
                cache_controller,
            );

        let layer = Box::new(galileo::layer::RasterTileLayer::new(
            galileo::TileScheme::web(18),
            tile_provider,
            None,
        ));

        let map = Arc::new(RwLock::new(galileo::map::Map::new(
            view,
            vec![layer],
            Some(messenger),
        )));

        GalileoState {
            input_handler,
            event_processor,
            renderer,
            map,
        }
    }

    pub fn about_to_wait(&self) {
        self.map.write().unwrap().animate();
    }

    pub fn resize(&self, size: PhysicalSize<u32>) {
        self.renderer
            .write()
            .expect("poisoned lock")
            .resize(Size::new(size.width, size.height));
        self.map
            .write()
            .expect("poisoned lock")
            .set_size(Size::new(size.width as f64, size.height as f64));
    }

    pub fn render(&self, wgpu_frame: &WgpuFrame<'_>) {
        let cast: Arc<RwLock<dyn galileo::render::Renderer>> = self.renderer.clone();

        let galileo_map = self.map.read().unwrap();
        galileo_map.load_layers(&cast);

        self.renderer
            .write()
            .expect("poisoned lock")
            .render_to_texture_view(&galileo_map, wgpu_frame.texture_view);
    }

    pub fn handle_event(&mut self, event: &winit::event::WindowEvent) {
        // Phone emulator in browsers works funny with scaling, using this code fixes it.
        // But my real phone works fine without it, so it's commented out for now, and probably
        // should be deleted later, when we know that it's not needed on any devices.

        // #[cfg(target_arch = "wasm32")]
        // let scale = window.scale_factor();
        //
        // #[cfg(not(target_arch = "wasm32"))]
        let scale = 1.0;

        if let Some(raw_event) = self.input_handler.process_user_input(event, scale) {
            let mut map = self.map.write().expect("poisoned lock");
            self.event_processor.handle(
                raw_event,
                &mut map,
                &(*self.renderer.read().expect("poisoned lock")),
            );
        }
    }
}
