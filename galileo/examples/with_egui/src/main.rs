#![allow(dead_code)]

mod run_ui;
mod state;

use winit::window::Window;
use with_egui::run;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let event_loop = winit::event_loop::EventLoop::new().unwrap();

    // TODO: refactor this to use winit 0.30 approach to create windows
    #[allow(deprecated)]
    let window = event_loop
        .create_window(Window::default_attributes().with_title("egui + galileo"))
        .unwrap();

    run(window, event_loop).await;
}
