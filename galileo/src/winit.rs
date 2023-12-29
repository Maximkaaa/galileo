use crate::control::{MouseButton, RawUserEvent, TouchEvent};
use crate::messenger::Messenger;
use galileo_types::cartesian::impls::point::Point2d;
use std::sync::Arc;
use winit::event::{ElementState, MouseScrollDelta, Touch, TouchPhase, WindowEvent};
use winit::window::Window;

#[derive(Debug, Default)]
pub struct WinitInputHandler {}

impl WinitInputHandler {
    pub fn process_user_input(&mut self, winit_event: &WindowEvent) -> Option<RawUserEvent> {
        match winit_event {
            WindowEvent::MouseInput { button, state, .. } => match state {
                ElementState::Pressed => Some(RawUserEvent::ButtonPressed(button.into())),
                ElementState::Released => Some(RawUserEvent::ButtonReleased(button.into())),
            },
            WindowEvent::CursorMoved { position, .. } => {
                let pointer_position = Point2d::new(position.x, position.y);
                Some(RawUserEvent::PointerMoved(pointer_position))
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let zoom = match delta {
                    MouseScrollDelta::LineDelta(_, dy) => *dy as f64,
                    MouseScrollDelta::PixelDelta(pos) => pos.y / 114.0,
                };
                if zoom.abs() < 0.0001 {
                    return None;
                }

                Some(RawUserEvent::MouseWheel(zoom))
            }
            WindowEvent::Touch(touch) => match touch.phase {
                TouchPhase::Started => Some(RawUserEvent::TouchStart(self.get_touch_event(touch))),
                TouchPhase::Moved => Some(RawUserEvent::TouchStart(self.get_touch_event(touch))),
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    Some(RawUserEvent::TouchStart(self.get_touch_event(touch)))
                }
            },
            _ => None,
        }
    }

    fn get_touch_event(&mut self, touch: &Touch) -> TouchEvent {
        TouchEvent {
            touch_id: touch.id,
            position: Point2d::new(touch.location.x, touch.location.y),
        }
    }
}

impl From<&winit::event::MouseButton> for MouseButton {
    fn from(value: &winit::event::MouseButton) -> Self {
        match value {
            winit::event::MouseButton::Left => MouseButton::Left,
            winit::event::MouseButton::Right => MouseButton::Right,
            winit::event::MouseButton::Middle => MouseButton::Middle,
            winit::event::MouseButton::Back => MouseButton::Other,
            winit::event::MouseButton::Forward => MouseButton::Other,
            winit::event::MouseButton::Other(_) => MouseButton::Other,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WinitMessenger {
    pub window: Arc<Window>,
}

impl WinitMessenger {
    pub fn new(window: Arc<Window>) -> Self {
        Self { window }
    }
}

impl Messenger for WinitMessenger {
    fn request_redraw(&self) {
        self.window.request_redraw();
    }
}
