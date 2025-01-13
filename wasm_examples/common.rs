use galileo_types::cartesian::Size;
use web_sys::HtmlElement;
use wasm_bindgen::JsCast;

pub async fn set_up() -> (HtmlElement, Size<u32>) {
    console_log::init_with_level(log::Level::Info).expect("Couldn't init logger");

    let web_window = web_sys::window().unwrap();
    let container: HtmlElement = web_window.document()
        .and_then(|doc| {
            doc.get_element_by_id("map")
        })
        .expect("Coundn't get map container")
        .dyn_into()
        .expect("Container is not an HtmlElement");
    let scale = web_window.device_pixel_ratio();

    let width = web_window.inner_width().unwrap().as_f64().unwrap() * scale;
    let height = web_window.inner_height().unwrap().as_f64().unwrap() * scale;

    let size = Size::new(width as u32, height as u32);

    log::info!("Window size is {size:?}");

    (container, size)
}

