//! Types that help using `Galileo` with `winit`.

use std::sync::Arc;

use galileo_types::cartesian::Point2;
use winit::event::{ElementState, MouseScrollDelta, Touch, TouchPhase, WindowEvent};
use winit::window::Window;

use crate::control::{MouseButton, RawUserEvent, TouchEvent};
use crate::messenger::Messenger;

/// Converts `winit` events into `Galileo` [`RawUserEvent`]s.
#[derive(Debug, Default)]
pub struct WinitInputHandler {}

impl WinitInputHandler {
    /// Convert `winit` event into `Galileo` event.
    pub fn process_user_input(
        &mut self,
        winit_event: &WindowEvent,
        scale: f64,
    ) -> Option<RawUserEvent> {
        match winit_event {
            WindowEvent::MouseInput { button, state, .. } => match state {
                ElementState::Pressed => Some(RawUserEvent::ButtonPressed(button.into())),
                ElementState::Released => Some(RawUserEvent::ButtonReleased(button.into())),
            },
            WindowEvent::CursorMoved { position, .. } => {
                let pointer_position = Point2::new(position.x / scale, position.y / scale);
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

                Some(RawUserEvent::Scroll(zoom))
            }
            WindowEvent::Touch(touch) => match touch.phase {
                TouchPhase::Started => {
                    Some(RawUserEvent::TouchStart(self.get_touch_event(touch, scale)))
                }
                TouchPhase::Moved => {
                    Some(RawUserEvent::TouchMove(self.get_touch_event(touch, scale)))
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    Some(RawUserEvent::TouchEnd(self.get_touch_event(touch, scale)))
                }
            },
            _ => None,
        }
    }

    fn get_touch_event(&mut self, touch: &Touch, scale: f64) -> TouchEvent {
        TouchEvent {
            touch_id: touch.id,
            position: Point2::new(touch.location.x / scale, touch.location.y / scale),
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

/// Messenger for a `winit` window.
#[derive(Debug, Clone)]
pub struct WinitMessenger {
    window: Arc<Window>,
}

impl WinitMessenger {
    /// Creates a new messenger.
    pub fn new(window: Arc<Window>) -> Self {
        Self { window }
    }
}

impl Messenger for WinitMessenger {
    fn request_redraw(&self) {
        self.window.request_redraw();
    }
}
