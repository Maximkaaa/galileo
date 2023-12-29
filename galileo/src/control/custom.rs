use crate::control::{EventPropagation, UserEvent, UserEventHandler};
use crate::map::Map;
use crate::render::Renderer;
use maybe_sync::{MaybeSend, MaybeSync};

pub trait EventHandler:
    (Fn(&UserEvent, &mut Map, &dyn Renderer) -> EventPropagation) + MaybeSend + MaybeSync
{
}

impl<T: Fn(&UserEvent, &mut Map, &dyn Renderer) -> EventPropagation> EventHandler for T where
    T: MaybeSync + MaybeSend
{
}

#[derive(Default)]
pub struct CustomEventHandler {
    input_handler: Option<Box<dyn EventHandler>>,
}

impl CustomEventHandler {
    pub fn set_input_handler(&mut self, handler: impl EventHandler + 'static) {
        self.input_handler = Some(Box::new(handler));
    }
}

impl UserEventHandler for CustomEventHandler {
    fn handle(&self, event: &UserEvent, map: &mut Map, backend: &dyn Renderer) -> EventPropagation {
        if let Some(handler) = &self.input_handler {
            handler(event, map, backend)
        } else {
            EventPropagation::Propagate
        }
    }
}
