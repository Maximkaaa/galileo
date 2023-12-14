use galileo::bounding_box::BoundingBox;
use galileo::control::custom::CustomEventHandler;
use galileo::control::event_processor::EventProcessor;
use galileo::control::map::MapController;
use galileo::control::{EventPropagation, MouseButton, UserEvent, UserEventHandler};
use galileo::layer::vector_tile::style::VectorTileStyle;
use galileo::layer::vector_tile::tile_provider::web_worker_provider::WebWorkerVectorTileProvider;
use galileo::layer::vector_tile::VectorTileLayer;
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

static mut MAP: Option<Arc<RwLock<galileo::map::Map>>> = None;

#[wasm_bindgen]
extern "C" {
    fn send_feature(layer: String, feature_type: String, feature: String);
}

#[wasm_bindgen]
pub fn set_style(style_json: JsValue) {
    let str = style_json.as_string().unwrap();
    let style = serde_json::from_str(&str).unwrap_or_else(|_| get_layer_style());
    unsafe {
        let map = MAP.as_ref().unwrap();
        let mut map = map.write().unwrap();
        map.layer_mut(0)
            .as_mut()
            .unwrap()
            .as_any_mut()
            .downcast_mut::<VectorTileLayer<WebWorkerVectorTileProvider>>()
            .unwrap()
            .update_style(style);
        map.redraw();
    };
}

fn get_layer_style() -> VectorTileStyle {
    serde_json::from_str(include_str!("../../../galileo/examples/data/vt_style.json")).unwrap()
}

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
    let style = get_layer_style();
    let layer = VectorTileLayer::<WebWorkerVectorTileProvider>::from_url(
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
    let mut map = galileo::map::Map::new(
        galileo::view::MapView::new(Point2d::new(0.0, 0.0), 156543.03392800014 / 8.0),
        vec![Box::new(layer)],
        messenger.clone(),
    );

    let map = Arc::new(RwLock::new(map));

    let mut custom_handler = CustomEventHandler::default();
    custom_handler.set_input_handler(move |ev, map| match ev {
        UserEvent::Click(MouseButton::Left, mouse_event) => {
            let resolution = map.view().resolution();
            let features = map
                .layer_mut(0)
                .as_mut()
                .unwrap()
                .as_any_mut()
                .downcast_mut::<VectorTileLayer<WebWorkerVectorTileProvider>>()
                .unwrap()
                .get_features_at(&mouse_event.map_pointer_position, resolution);

            for (layer, feature) in features {
                log::info!("{layer}, {:?}", feature.properties);
                send_feature(
                    layer.clone(),
                    format!("maybe polygon"),
                    serde_json::to_string(&feature.properties).unwrap(),
                );
            }

            EventPropagation::Stop
        }
        _ => EventPropagation::Propagate,
    });

    let mut input_handler = WinitInputHandler::default();
    let mut controller = MapController::default();
    let mut event_processor = EventProcessor::default();
    event_processor.add_handler(custom_handler);
    event_processor.add_handler(controller);

    unsafe {
        MAP = Some(map.clone());
    }

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
                            map.write()
                                .unwrap()
                                .set_size(Size::new(size.width as f64, size.height as f64));
                        }
                        WindowEvent::RedrawRequested => {
                            let map = map.read().unwrap();
                            let cast: Arc<RwLock<dyn Renderer>> = backend.clone();
                            map.load_layers(&cast);
                            backend.read().unwrap().render(&map).unwrap();
                        }
                        other => {
                            if let Some(raw_event) = input_handler.process_user_input(&other) {
                                let size = backend.read().unwrap().size();
                                event_processor.handle(raw_event, &mut map.write().unwrap());
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

async fn sleep(duration: i32) {
    let mut cb = |resolve: js_sys::Function, reject: js_sys::Function| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, duration);
    };

    let p = js_sys::Promise::new(&mut cb);

    wasm_bindgen_futures::JsFuture::from(p).await.unwrap();
}

pub fn tile_scheme() -> TileScheme {
    const ORIGIN: Point2d = Point2d::new(-20037508.342787, 20037508.342787);
    const TOP_RESOLUTION: f64 = 156543.03392800014 / 4.0;

    let mut lods = vec![Lod::new(TOP_RESOLUTION, 0).unwrap()];
    for i in 1..17 {
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
