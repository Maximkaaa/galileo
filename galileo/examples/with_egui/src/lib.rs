use std::sync::Arc;

use winit::{
    event::{Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

mod run_ui;
mod state;

pub async fn run(window: Window, event_loop: EventLoop<()>) {
    let window = Arc::new(window);

    let mut state = state::State::new(Arc::clone(&window)).await;

    let _ = event_loop.run(move |event, ewlt| {
        ewlt.set_control_flow(ControlFlow::Wait);

        match event {
            Event::AboutToWait => {
                state.about_to_wait();
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window().id() => {
                match event {
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                logical_key:
                                    winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape),
                                ..
                            },
                        ..
                    } => ewlt.exit(),
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::RedrawRequested => match state.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            state.resize(state.size)
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => ewlt.exit(),
                        Err(wgpu::SurfaceError::Timeout) => {
                            // Ignore timeouts.
                        }
                    },
                    other => {
                        state.handle_event(other);
                        window.request_redraw();
                        return;
                    }
                };
                state.handle_event(event);
                window.request_redraw();
            }
            _ => {}
        }
    });
}
