use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoop;
use winit::platform::web::WindowExtWebSys;
use winit::window::{Window, WindowBuilder};

pub async fn set_up() -> (Window, EventLoop<()>) {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Info).expect("Couldn't init logger");

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let window = window;

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
    let _ = window.request_inner_size(PhysicalSize::new(
        web_window.inner_width().unwrap().as_f64().unwrap(),
        web_window.inner_height().unwrap().as_f64().unwrap(),
    ));

    (window, event_loop)
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
