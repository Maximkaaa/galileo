use galileo::bounding_box::BoundingBox;
use galileo::control::custom::CustomEventHandler;
use galileo::control::event_processor::EventProcessor;
use galileo::control::map::MapController;
use galileo::control::{EventPropagation, MouseButton, UserEvent};
use galileo::layer::vector_tile::style::VectorTileStyle;
use galileo::layer::vector_tile::tile_provider::rayon_provider::RayonProvider;
use galileo::layer::vector_tile::VectorTileLayer;
use galileo::lod::Lod;
use galileo::primitives::Point2d;
use galileo::render::Renderer;
use galileo::tile_scheme::{TileScheme, VerticalDirection};
use galileo::winit::{WinitInputHandler, WinitMessenger};
use std::path::Path;
use std::sync::{Arc, RwLock};
use winit::event_loop::ControlFlow;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

const STYLE: &str = "galileo/examples/data/vt_style.json";

fn get_layer_style() -> Option<VectorTileStyle> {
    serde_json::from_reader(std::fs::File::open(STYLE).ok()?).ok()
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let window = Arc::new(window);

    let messenger = WinitMessenger::new(window.clone());

    let backend = galileo::render::wgpu::WgpuRenderer::create(&window).await;
    let style = get_layer_style().unwrap();
    let vt_layer = VectorTileLayer::<RayonProvider>::from_url(
        |index| {
            format!(
                "https://d1zqyi8v6vm8p9.cloudfront.net/planet/{}/{}/{}.mvt",
                index.z, index.x, index.y
            )
        },
        style,
        messenger.clone(),
        tile_scheme(),
    );

    let map = galileo::map::Map::new(
        galileo::view::MapView {
            position: Point2d::new(0.0, 0.0),
            // resolution: 8192.0,
            resolution: 156543.03392800014 / 4.0,
        },
        vec![Box::new(vt_layer)],
        messenger.clone(),
    );

    let map = Arc::new(RwLock::new(map));

    let window_clone = window.clone();
    let map_clone = map.clone();
    let mut watcher = notify::recommended_watcher(move |_res| {
        if let Some(style) = get_layer_style() {
            map_clone
                .write()
                .unwrap()
                .layer_mut(0)
                .as_mut()
                .unwrap()
                .as_any_mut()
                .downcast_mut::<VectorTileLayer<RayonProvider>>()
                .unwrap()
                .update_style(style);
            window_clone.request_redraw()
        }
    })
    .unwrap();

    use notify::Watcher;
    watcher
        .watch(Path::new(STYLE), notify::RecursiveMode::NonRecursive)
        .unwrap();

    let mut custom_handler = CustomEventHandler::default();
    custom_handler.set_input_handler(move |ev, map| match ev {
        UserEvent::Click(MouseButton::Left, mouse_event) => {
            let resolution = map.view().resolution;
            let features = map
                .layer_mut(0)
                .as_mut()
                .unwrap()
                .as_any_mut()
                .downcast_mut::<VectorTileLayer<RayonProvider>>()
                .unwrap()
                .get_features_at(&mouse_event.map_pointer_position, resolution);

            for (layer, feature) in features {
                println!("{layer}, {:?}", feature.properties);
            }

            EventPropagation::Stop
        }
        _ => EventPropagation::Propagate,
    });

    let mut input_handler = WinitInputHandler::default();
    let controller = MapController::default();
    let mut event_processor = EventProcessor::default();
    event_processor.add_handler(custom_handler);
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
                            let map = map.read().unwrap();
                            let cast: Arc<RwLock<dyn Renderer>> = backend.clone();
                            map.load_layers(backend.read().unwrap().size(), &cast);
                            backend.read().unwrap().render(&map).unwrap();
                        }
                        other => {
                            if let Some(raw_event) = input_handler.process_user_input(&other) {
                                let size = backend.read().unwrap().size();
                                event_processor.handle(raw_event, &mut map.write().unwrap(), size);
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

pub fn tile_scheme() -> TileScheme {
    const ORIGIN: Point2d = Point2d::new(-20037508.342787, 20037508.342787);
    const TOP_RESOLUTION: f64 = 156543.03392800014 / 4.0;

    let mut lods = vec![Lod::new(TOP_RESOLUTION, 0).unwrap()];
    for i in 1..16 {
        lods.push(Lod::new(lods[(i - 1) as usize].resolution() / 2.0, i).unwrap());
    }

    TileScheme {
        origin: ORIGIN,
        bounds: BoundingBox::new(
            -20037508.342787,
            -20037508.342787,
            20037508.342787,
            20037508.342787,
        ),
        lods: lods.into_iter().collect(),
        tile_width: 1024,
        tile_height: 1024,
        y_direction: VerticalDirection::TopToBottom,
        max_tile_scale: 8.0,
        cycle_x: true,
    }
}
