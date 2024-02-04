mod run_ui;
mod state;

use with_egui::run;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let window = winit::window::WindowBuilder::new()
        .with_title("egui + galileo")
        .build(&event_loop)
        .unwrap();

    run(window, event_loop).await;
}
