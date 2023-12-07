use crate::data::{load_countries, Country};
use galileo::control::custom::CustomEventHandler;
use galileo::control::event_processor::EventProcessor;
use galileo::control::map::MapController;
use galileo::control::{EventPropagation, MouseButton, UserEvent};
use galileo::layer::feature::{FeatureLayer, SimplePolygonSymbol, Symbol};
use galileo::layer::Layer;
use galileo::messenger::Messenger;
use galileo::primitives::{Color, Point2d};
use galileo::render::{Canvas, RenderBundle, UnpackedBundle};
use galileo::winit::{WinitInputHandler, WinitMessenger};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use wasm_bindgen::prelude::*;
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::platform::web::WindowExtWebSys;
use winit::window::WindowBuilder;

#[path = "../../../galileo/examples/data/mod.rs"]
mod data;

#[wasm_bindgen]
pub async fn init() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Info).expect("Couldn't init logger");

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        // .with_maximized(true)
        .build(&event_loop)
        .unwrap();
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

    let messenger = WinitMessenger::new(window.clone());

    let mut backend = galileo::render::wgpu::WgpuRenderer::create(&window).await;
    let backend = Arc::new(Mutex::new(backend));

    let countries = load_countries();
    let feature_layer = FeatureLayer::new(countries, CountrySymbol {});
    let feature_layer = Arc::new(RwLock::new(feature_layer));

    let mut map = galileo::map::Map::new(
        galileo::view::MapView {
            position: Point2d::new(0.0, 0.0),
            resolution: 156543.03392800014 / 8.0,
        },
        vec![Box::new(feature_layer.clone())],
        messenger.clone(),
    );

    let layer_clone = feature_layer.clone();
    let mut custom_handler = CustomEventHandler::default();
    let selected_index = Arc::new(AtomicUsize::new(usize::MAX));

    let backend_copy = backend.clone();
    custom_handler.set_input_handler(move |ev, map| {
        if let UserEvent::Click(button, event) = ev {
            if *button == MouseButton::Left {
                let layer = layer_clone.write().unwrap();

                for (_idx, feature) in
                    layer.get_features_at(&event.map_pointer_position, map.view().resolution * 2.0)
                {
                    log::info!("Found {} with bbox {:?}", feature.name, feature.bbox);
                }

                return EventPropagation::Stop;
            }
        }

        if let UserEvent::PointerMoved(event) = ev {
            let mut layer = layer_clone.write().unwrap();

            let mut to_update = vec![];

            let mut new_selected = usize::MAX;
            if let Some((index, feature)) = layer
                .get_features_at_mut(&event.map_pointer_position, map.view().resolution * 2.0)
                .first_mut()
            {
                if *index == selected_index.load(Ordering::Relaxed) {
                    return EventPropagation::Stop;
                }
                feature.is_selected = true;
                new_selected = *index;
                to_update.push(*index);
            }

            let selected = selected_index.swap(new_selected, Ordering::Relaxed);
            if selected != usize::MAX {
                let feature = layer.features_mut().skip(selected).next().unwrap();
                feature.is_selected = false;
                to_update.push(selected);
            }

            if !to_update.is_empty() {
                let backend = backend_copy.lock().unwrap();
                layer.update_features(&to_update, &backend);
                messenger.request_redraw();
            }

            return EventPropagation::Stop;
        }

        EventPropagation::Propagate
    });

    let mut input_handler = WinitInputHandler::default();
    let controller = MapController::default();
    let mut event_processor = EventProcessor::default();
    event_processor.add_handler(custom_handler);
    event_processor.add_handler(controller);

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
                            backend.lock().unwrap().resize(size);
                        }
                        WindowEvent::RedrawRequested => {
                            backend.lock().unwrap().render(&map).unwrap();
                        }
                        other => {
                            if let Some(raw_event) = input_handler.process_user_input(&other) {
                                let size = backend.lock().unwrap().size();
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

async fn sleep(duration: i32) {
    let mut cb = |resolve: js_sys::Function, reject: js_sys::Function| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, duration);
    };

    let p = js_sys::Promise::new(&mut cb);

    wasm_bindgen_futures::JsFuture::from(p).await.unwrap();
}

struct CountrySymbol {}

impl CountrySymbol {
    fn get_polygon_symbol(&self, feature: &Country) -> SimplePolygonSymbol {
        let stroke_color = feature.color;
        let fill_color = Color {
            a: if feature.is_selected() { 255 } else { 200 },
            ..stroke_color
        };
        SimplePolygonSymbol {
            fill_color,
            stroke_color,
            stroke_width: 1.0,
            stroke_offset: -0.5,
        }
    }
}

impl Symbol<Country> for CountrySymbol {
    fn render(&self, feature: &Country, bundle: &mut Box<dyn RenderBundle>) -> Vec<usize> {
        let mut ids = vec![];
        for polygon in &feature.geometry {
            ids.append(&mut self.get_polygon_symbol(feature).render(polygon, bundle))
        }

        ids
    }

    fn update(
        &self,
        feature: &Country,
        render_ids: &[usize],
        bundle: &mut Box<dyn UnpackedBundle>,
    ) {
        let renders_by_feature = render_ids.len() / feature.geometry.len();
        let mut next_index = 0;
        for geom in &feature.geometry {
            self.get_polygon_symbol(feature).update(
                geom,
                &render_ids[next_index..next_index + renders_by_feature],
                bundle,
            );

            next_index += renders_by_feature;
        }
    }
}
