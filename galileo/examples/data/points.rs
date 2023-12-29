use galileo::control::event_processor::EventProcessor;
use galileo::control::map::MapController;
use galileo::layer::feature::{CirclePointSymbol, FeatureLayer};
use galileo::primitives::Color;
use galileo::render::Renderer;
use galileo::winit::{WinitInputHandler, WinitMessenger};
use galileo_types::cartesian::impls::point::Point2d;
use galileo_types::cartesian::size::Size;
use nalgebra::Point3;
use std::sync::{Arc, RwLock};
use winit::event_loop::ControlFlow;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};
// todo: temporary moved out of examples until 3d points are figured out
fn generate_points() -> Vec<Vec<Point3<f64>>> {
    let mut points = vec![];
    for x in -50..50 {
        for y in -50..50 {
            for z in 0..150 {
                points.push(Point3::new(
                    x as f64 * 10000.0,
                    y as f64 * 10000.0,
                    z as f64 * 10000.0,
                ))
            }
        }
    }
    vec![points]
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
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

    let points = generate_points();
    let feature_layer = FeatureLayer::new(
        points,
        CirclePointSymbol {
            size: 3.0,
            color: Color::rgba(226, 184, 34, 255),
        },
    );

    let mut map = galileo::map::Map::new(
        galileo::view::MapView::new_projected(&Point2d::new(0.0, 0.0), 156543.03392800014 / 4.0),
        vec![Box::new(osm), Box::new(feature_layer)],
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
