use galileo::bounding_box::BoundingBox;
use galileo::control::event_processor::EventProcessor;
use galileo::control::map::MapController;
use galileo::control::{EventPropagation, MouseButton, UserEvent, UserEventHandler};
use galileo::layer::Layer;
use galileo::lod::Lod;
use galileo::primitives::Point2d;
use galileo::render::Renderer;
use galileo::tile_scheme::{TileScheme, VerticalDirection};
use galileo::winit::{WinitInputHandler, WinitMessenger};
use galileo_types::size::Size;
use std::sync::{Arc, RwLock};
use wasm_bindgen::prelude::*;
use winit::dpi::PhysicalSize;
use winit::event_loop::ControlFlow;
use winit::platform::web::WindowExtWebSys;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

#[wasm_bindgen]
pub async fn init() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Info).expect("Couldn't init logger");

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let window = Arc::new(window);

    web_sys::window()
        .and_then(|win| win.document())
        .and_then(|doc| {
            let dst = doc.get_element_by_id("map")?;
            let canvas = web_sys::Element::from(window.canvas()?);
            dst.append_child(&canvas).ok()?;

            Some(())
        })
        .expect("Couldn't create canvas");

    sleep(10).await;

    let web_window = web_sys::window().unwrap();
    window.request_inner_size(PhysicalSize::new(
        web_window.inner_width().unwrap().as_f64().unwrap(),
        web_window.inner_height().unwrap().as_f64().unwrap(),
    ));

    log::info!("Window size is {:?}", window.inner_size());

    let messenger = WinitMessenger::new(window.clone());

    let mut backend = galileo::render::wgpu::WgpuRenderer::create(&window).await;
    let layer = galileo::layer::raster_tile::RasterTileLayer::from_url(
        |index| {
            format!(
                "https://tile.openstreetmap.org/{}/{}/{}.png",
                index.z, index.x, index.y
            )
        },
        messenger.clone(),
    );

    let mut map = galileo::map::Map::new(
        galileo::view::MapView::new(Point2d::new(0.0, 0.0), 156543.03392800014 / 8.0),
        vec![Box::new(layer)],
        messenger.clone(),
    );

    let mut input_handler = WinitInputHandler::default();
    let mut controller = MapController::default();
    let mut event_processor = EventProcessor::default();
    event_processor.add_handler(controller);

    let mut backend = Arc::new(RwLock::new(backend));

    event_loop
        .run(move |event, target| {
            target.set_control_flow(ControlFlow::Wait);

            match event {
                Event::WindowEvent { event, window_id } if window_id == window.id() => {
                    match event {
                        WindowEvent::CloseRequested => {
                            target.exit();
                        }
                        WindowEvent::Resized(size) => {
                            backend.write().unwrap().resize(size);
                            map.set_size(Size::new(size.width as f64, size.height as f64));
                        }
                        WindowEvent::RedrawRequested => {
                            let cast: Arc<RwLock<dyn Renderer>> = backend.clone();
                            map.load_layers(&cast);
                            backend.read().unwrap().render(&map).unwrap();
                        }
                        other => {
                            if let Some(raw_event) = input_handler.process_user_input(&other) {
                                let size = backend.read().unwrap().size();
                                event_processor.handle(raw_event, &mut map);
                            }
                        }
                    }
                }
                Event::AboutToWait => {
                    map.animate();
                }
                _ => (),
            }
        })
        .unwrap();
}

async fn sleep(duration: i32) {
    let mut cb = |resolve: js_sys::Function, reject: js_sys::Function| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, duration);
    };

    let p = js_sys::Promise::new(&mut cb);

    wasm_bindgen_futures::JsFuture::from(p).await.unwrap();
}
