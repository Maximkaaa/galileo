use crate::map::Map;
use crate::render::Renderer;
use galileo_types::cartesian::impls::point::Point2d;
use nalgebra::Vector2;

pub mod custom;
pub mod event_processor;
pub mod map;

pub trait UserEventHandler {
    fn handle(&self, event: &UserEvent, map: &mut Map, backend: &dyn Renderer) -> EventPropagation;
}

pub enum RawUserEvent {
    ButtonPressed(MouseButton),
    ButtonReleased(MouseButton),
    PointerMoved(Point2d),
    Scroll(f64),
    TouchStart(TouchEvent),
    TouchMove(TouchEvent),
    TouchEnd(TouchEvent),
}

#[derive(Debug, Clone)]
pub enum UserEvent {
    ButtonPressed(MouseButton, MouseEvent),
    ButtonReleased(MouseButton, MouseEvent),
    Click(MouseButton, MouseEvent),
    DoubleClick(MouseButton, MouseEvent),
    PointerMoved(MouseEvent),

    DragStarted(MouseButton, MouseEvent),
    Drag(MouseButton, Vector2<f64>, MouseEvent),
    DragEnded(MouseButton, MouseEvent),

    Scroll(f64, MouseEvent),
    Zoom(f64, Point2d),
}

pub enum EventPropagation {
    Propagate,
    Stop,
    Consume,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    Other,
}

#[derive(Debug, Clone)]
pub struct MouseEvent {
    pub screen_pointer_position: Point2d,
    pub buttons: MouseButtonsState,
}

pub type TouchId = u64;

#[derive(Debug, Clone)]
pub struct TouchEvent {
    pub touch_id: TouchId,
    pub position: Point2d,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MouseButtonState {
    Pressed,
    Released,
}

#[derive(Debug, Copy, Clone)]
pub struct MouseButtonsState {
    pub left: MouseButtonState,
    pub middle: MouseButtonState,
    pub right: MouseButtonState,
}

impl MouseButtonsState {
    pub(crate) fn set_pressed(&mut self, button: MouseButton) {
        self.set_state(button, MouseButtonState::Pressed);
    }

    pub(crate) fn set_released(&mut self, button: MouseButton) {
        self.set_state(button, MouseButtonState::Released);
    }

    fn set_state(&mut self, button: MouseButton, state: MouseButtonState) {
        match button {
            MouseButton::Left => self.left = state,
            MouseButton::Middle => self.middle = state,
            MouseButton::Right => self.right = state,
            MouseButton::Other => {}
        }
    }

    fn single_pressed(&self) -> Option<MouseButton> {
        let mut button = None;
        if self.left == MouseButtonState::Pressed && button.replace(MouseButton::Left).is_some() {
            return None;
        }
        if self.middle == MouseButtonState::Pressed && button.replace(MouseButton::Middle).is_some()
        {
            return None;
        }
        if self.right == MouseButtonState::Pressed && button.replace(MouseButton::Right).is_some() {
            return None;
        }

        button
    }
}

impl Default for MouseButtonsState {
    fn default() -> Self {
        Self {
            left: MouseButtonState::Released,
            middle: MouseButtonState::Released,
            right: MouseButtonState::Released,
        }
    }
}
