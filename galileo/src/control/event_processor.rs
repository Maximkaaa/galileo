use crate::control::{
    EventPropagation, MouseButtonsState, MouseEvent, RawUserEvent, UserEvent, UserEventHandler,
};
use crate::map::Map;
use galileo_types::cartesian::impls::point::Point2d;
use galileo_types::cartesian::traits::cartesian_point::CartesianPoint2d;
use web_time::SystemTime;

const DRAG_THRESHOLD: f64 = 3.0;
const CLICK_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(200);
const DBL_CLICK_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(500);

pub struct EventProcessor {
    handlers: Vec<Box<dyn UserEventHandler>>,
    pointer_position: Point2d,
    pointer_pressed_position: Point2d,

    buttons_state: MouseButtonsState,

    last_pressed_time: SystemTime,
    last_click_time: SystemTime,

    drag_target: Option<usize>,
}

impl Default for EventProcessor {
    fn default() -> Self {
        Self {
            handlers: vec![],
            pointer_position: Default::default(),
            pointer_pressed_position: Default::default(),
            buttons_state: Default::default(),
            last_pressed_time: SystemTime::UNIX_EPOCH,
            last_click_time: SystemTime::UNIX_EPOCH,
            drag_target: None,
        }
    }
}

impl EventProcessor {
    pub fn add_handler(&mut self, handler: impl UserEventHandler + 'static) {
        self.handlers.push(Box::new(handler));
    }

    pub fn handle(&mut self, event: RawUserEvent, map: &mut Map) {
        if let Some(user_events) = self.process(event) {
            for user_event in user_events {
                let mut drag_start_target = None;

                let delta = self.pointer_position - self.pointer_pressed_position;
                let mouse_event = self.get_mouse_event();

                for (index, handler) in self.handlers.iter_mut().enumerate() {
                    if matches!(user_event, UserEvent::Drag(..) | UserEvent::DragEnded(..)) {
                        if let Some(target) = &self.drag_target {
                            if index != *target {
                                continue;
                            }
                        } else {
                            continue;
                        }
                    }

                    match handler.handle(&user_event, map) {
                        EventPropagation::Propagate => {}
                        EventPropagation::Stop => break,
                        EventPropagation::Consume => {
                            if let UserEvent::DragStarted(button, _) = user_event {
                                drag_start_target = Some(index);

                                handler.handle(
                                    &UserEvent::Drag(button, delta.into(), mouse_event),
                                    map,
                                );
                            }

                            break;
                        }
                    }
                }

                if drag_start_target.is_some() {
                    self.drag_target = drag_start_target;
                }

                if matches!(user_event, UserEvent::DragEnded(..)) {
                    self.drag_target = None;
                }
            }
        }
    }

    fn process(&mut self, event: RawUserEvent) -> Option<Vec<UserEvent>> {
        let now = SystemTime::now();
        match event {
            RawUserEvent::ButtonPressed(button) => {
                self.buttons_state.set_pressed(button);
                self.last_pressed_time = now;
                self.pointer_pressed_position = self.pointer_position;

                Some(vec![UserEvent::ButtonPressed(
                    button,
                    self.get_mouse_event(),
                )])
            }
            RawUserEvent::ButtonReleased(button) => {
                self.buttons_state.set_released(button);
                let mut events = vec![UserEvent::ButtonReleased(button, self.get_mouse_event())];

                if (now.duration_since(self.last_pressed_time)).unwrap_or_default() < CLICK_TIMEOUT
                {
                    events.push(UserEvent::Click(button, self.get_mouse_event()));

                    if (now.duration_since(self.last_click_time)).unwrap_or_default()
                        < DBL_CLICK_TIMEOUT
                    {
                        events.push(UserEvent::DoubleClick(button, self.get_mouse_event()));
                    }

                    self.last_click_time = now;

                    if self.drag_target.take().is_some() {
                        events.push(UserEvent::DragEnded(button, self.get_mouse_event()));
                    }
                }

                Some(events)
            }
            RawUserEvent::PointerMoved(position) => {
                let prev_position = self.pointer_position;
                self.pointer_position = position;

                let mut events = vec![UserEvent::PointerMoved(self.get_mouse_event())];
                if let Some(button) = self.buttons_state.single_pressed() {
                    if self.drag_target.is_none()
                        && position.taxicab_distance(&self.pointer_pressed_position)
                            > DRAG_THRESHOLD
                    {
                        events.push(UserEvent::DragStarted(
                            button,
                            self.get_mouse_event_pos(self.pointer_pressed_position),
                        ));
                    }

                    if self.drag_target.is_some() {
                        events.push(UserEvent::Drag(
                            button,
                            (self.pointer_position - prev_position).into(),
                            self.get_mouse_event(),
                        ));
                    }
                }

                Some(events)
            }
            RawUserEvent::MouseWheel(delta) => {
                Some(vec![UserEvent::Zoom(delta, self.get_mouse_event())])
            }
            // todo
            RawUserEvent::TouchStart(_touch) => None,
            RawUserEvent::TouchMove(_touch) => None,
            RawUserEvent::TouchEnd(_touch) => None,
        }
    }

    fn get_mouse_event(&self) -> MouseEvent {
        self.get_mouse_event_pos(self.pointer_position)
    }

    fn get_mouse_event_pos(&self, screen_pointer_position: Point2d) -> MouseEvent {
        MouseEvent {
            screen_pointer_position,
            buttons: self.buttons_state,
        }
    }
}
