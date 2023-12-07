use galileo::control::event_processor::EventProcessor;
use galileo::control::map::MapController;
use galileo::layer::feature::{CirclePointSymbol, FeatureLayer};
use galileo::layer::Layer;
use galileo::primitives::{Color, Point2d};
use galileo::render::Renderer;
use galileo::winit::{WinitInputHandler, WinitMessenger};
use std::sync::{Arc, RwLock};
use winit::event_loop::ControlFlow;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

#[tokio::main]
async fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        // .with_maximized(true)
        .build(&event_loop)
        .unwrap();
    let window = Arc::new(window);

    let messenger = WinitMessenger::new(window.clone());

    let backend = galileo::render::wgpu::WgpuRenderer::create(&window).await;
    let osm = galileo::layer::raster_tile::RasterTileLayer::from_url(
        |index| {
            format!(
                "https://tile.openstreetmap.org/{}/{}/{}.png",
                index.z, index.x, index.y
            )
        },
        messenger.clone(),
    );

    let mut map = galileo::map::Map::new(
        galileo::view::MapView {
            position: Point2d::new(0.0, 0.0),
            resolution: 156543.03392800014 / 4.0,
        },
        vec![Box::new(osm)],
        messenger.clone(),
    );

    let mut input_handler = WinitInputHandler::default();
    let controller = MapController::default();
    let mut event_processor = EventProcessor::default();
    event_processor.add_handler(controller);

    let backend = Arc::new(RwLock::new(backend));

    event_loop
        .run(move |event, target| {
            target.set_control_flow(ControlFlow::Wait);

            match event {
                Event::WindowEvent { event, window_id } if window_id == window.id() => {
                    match event {
                        WindowEvent::CloseRequested => {
                            eprintln!("The close button was pressed; stopping");
                            target.exit();
                        }
                        WindowEvent::Resized(size) => {
                            backend.write().unwrap().resize(size);
                        }
                        WindowEvent::RedrawRequested => {
                            let cast: Arc<RwLock<dyn Renderer>> = backend.clone();
                            map.load_layers(backend.read().unwrap().size(), &cast);
                            backend.read().unwrap().render(&map).unwrap();
                        }
                        other => {
                            if let Some(raw_event) = input_handler.process_user_input(&other) {
                                let size = backend.read().unwrap().size();
                                event_processor.handle(raw_event, &mut map, size);
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
