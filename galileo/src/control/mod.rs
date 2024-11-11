//! This module contains traits and structs that provide interactivity of a Galileo map.
//!
//! User interaction handling is done in several steps:
//! 1. OS event is converted to a common [`RawUserEvent`] enum. For example, apps that use `winit` can use
//!    [`WinitInputHandler`](crate::winit::WinitInputHandler) to convert [winit::event::WindowEvent] into `RawUserEvent`.
//! 2. `RawUserEvent` is given to the [`EventProcessor`], that converts it into a [`UserEvent`]. `EventProcessor`
//!    keeps track of input state (which keys, modifiers and mouse buttons) are pressed, and provides a more convenient
//!    way to handle user interactions for the application.
//! 3. `EventProcessor` has a list of [`UserEventHandler`]s, which change the state of application based on the events.
//!
//! To write a user interaction logic, the app must provide an implementation of [`UserEventHandler`] trait and add it
//! to the `EventProcessor` handler list.

use crate::map::Map;
use galileo_types::cartesian::Point2d;
use maybe_sync::{MaybeSend, MaybeSync};
use nalgebra::Vector2;

mod event_processor;
mod map;

pub use event_processor::EventProcessor;
pub use map::MapController;

/// User input handler.
pub trait UserEventHandler {
    /// Handle the event.
    fn handle(&self, event: &UserEvent, map: &mut Map) -> EventPropagation;
}

impl<T: for<'a> Fn(&'a UserEvent, &'a mut Map) -> EventPropagation> UserEventHandler for T
where
    T: MaybeSync + MaybeSend,
{
    fn handle(&self, event: &UserEvent, map: &mut Map) -> EventPropagation {
        self(event, map)
    }
}

/// Raw user interaction event. This type is an intermediate step between OS event and an event that will be processed
/// by the application. It does not provide any state information, as not all supported platforms give this information
/// together with the event. Instead, the input state information is stored in the [`EventProcessor`] struct, which
/// can combine `RawUserEvent` with the state to produce [`UserEvent`] which is then given to the application.
pub enum RawUserEvent {
    /// A mouse button was pressed.
    ButtonPressed(MouseButton),
    /// A mouse button was released.
    ButtonReleased(MouseButton),
    /// Mouse pointer was moved to the given screen pixel position.
    PointerMoved(Point2d),
    /// Scroll was called (by a mouse wheel or touch pad scrolling). The number is the number of lines that the event
    /// would scroll if it was scrolling a text.
    Scroll(f64),
    /// New touch started.
    TouchStart(TouchEvent),
    /// Existing touch moved.
    TouchMove(TouchEvent),
    /// Existing touch was released.
    TouchEnd(TouchEvent),
}

/// User interaction event. This is the main type that the application would use through [`UserEventHandler`]s.
#[derive(Debug, Clone)]
pub enum UserEvent {
    /// A mouse button was pressed.
    ButtonPressed(MouseButton, MouseEvent),
    /// A mouse button was released.
    ButtonReleased(MouseButton, MouseEvent),
    /// A mouse button was clicked. This event is fired right after the [`UserEvent::ButtonReleased`] event if the
    /// release was shortly after the press event (configured in [`EventProcessor`]).
    Click(MouseButton, MouseEvent),
    /// A double click was done. This event is fired right after the second [`UserEvent::Click`] event if the second
    /// click was done shortly after the first click (configured in [`EventProcessor`]).
    DoubleClick(MouseButton, MouseEvent),
    /// Mouse pointer moved.
    PointerMoved(MouseEvent),

    /// Drag started (user pressed a mouse button and moves the pointer around without releasing the button).
    ///
    /// This event is also fired when a single-finger touch is moved around.
    DragStarted(MouseButton, MouseEvent),

    /// Mouse pointer moved after drag started was consumed.
    Drag(MouseButton, Vector2<f64>, MouseEvent),

    /// Mouse button was released while dragging.
    DragEnded(MouseButton, MouseEvent),

    /// Scroll event is called. The number is number of text lines the scroll is requested for. This is then converted
    /// into zoom delta based on [`EventProcessor`] zoom speed configuration.
    Scroll(f64, MouseEvent),

    /// Zoom is called around a point. This is different from [`UserEvent::Scroll`], as it is not produced by a mouse
    /// but rather by multi-tough gestures. The first parameter is zoom delta value.
    Zoom(f64, Point2d),
}

/// Value returned by an [`UserEventHandler`] to indicate the status of the event.
pub enum EventPropagation {
    /// Event should be propagated to the next handler.
    Propagate,
    /// Event should not be propagated to the next handler.
    Stop,
    /// Event should not be propagated to the next handler, and the current event handler should be considered the
    /// owner of the event. This is used, for example, to indicate, that the handler wants to take ownership of
    /// the [`UserEvent::DragStarted`], so that all consequent drag events are only processed by this handler.
    Consume,
}

/// Mouse button enum.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MouseButton {
    /// The button you click when you want to shoot.
    Left,
    /// the button you click when you want to reload.
    Middle,
    /// The button you click when you want to hit with a rifle handle.
    Right,
    /// The button you click when you are a pro gamer and want to look cool.
    Other,
}

/// State of the mouse at the moment of the event.
#[derive(Debug, Clone)]
pub struct MouseEvent {
    /// Pointer position on the screen in pixels from the top-left corner.
    pub screen_pointer_position: Point2d,
    /// State of the mouse buttons.
    pub buttons: MouseButtonsState,
}

/// Id of the current touch.
pub type TouchId = u64;

/// Details of a touch event.
#[derive(Debug, Clone)]
pub struct TouchEvent {
    /// Id of the touch. Id is valid and unique only until the touch is ended. After that a new touch can have the same
    /// id.
    pub touch_id: TouchId,
    /// Position of the touch on the screen in pixels from the top-left corner.
    pub position: Point2d,
}

/// State of a mouse button.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MouseButtonState {
    /// Button is pressed.
    Pressed,
    /// Button is not pressed.
    Released,
}

/// State of all mouse buttons.
#[derive(Debug, Copy, Clone)]
pub struct MouseButtonsState {
    /// State of the left mouse button.
    pub left: MouseButtonState,
    /// State of the middle mouse button.
    pub middle: MouseButtonState,
    /// State of the right mouse button.
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
