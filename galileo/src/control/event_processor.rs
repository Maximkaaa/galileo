use crate::control::{
    EventPropagation, MouseButtonsState, MouseEvent, RawUserEvent, UserEvent, UserEventHandler,
};
use crate::map::Map;
use crate::primitives::Size;
use galileo_types::vec::Vec2d;
use galileo_types::{CartesianPoint2d, Point2d};
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

    pub fn handle(&mut self, event: RawUserEvent, map: &mut Map, screen_size: Size) {
        if let Some(user_events) = self.process(event, map, screen_size) {
            for user_event in user_events {
                let mut drag_start_target = None;

                let delta = self.get_map_delta(map, screen_size, self.pointer_pressed_position);
                let mouse_event = self.get_mouse_event(map, screen_size);

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

                                handler.handle(&UserEvent::Drag(button, delta, mouse_event), map);
                            }

                            break;
                        }
                    }
                }

                if drag_start_target.is_some() {
                    self.drag_target = drag_start_target;
                }
            }
        }
    }

    fn process(
        &mut self,
        event: RawUserEvent,
        map: &Map,
        screen_size: Size,
    ) -> Option<Vec<UserEvent>> {
        let now = SystemTime::now();
        match event {
            RawUserEvent::ButtonPressed(button) => {
                self.buttons_state.set_pressed(button);
                self.last_pressed_time = now;
                self.pointer_pressed_position = self.pointer_position;

                Some(vec![UserEvent::ButtonPressed(
                    button,
                    self.get_mouse_event(map, screen_size),
                )])
            }
            RawUserEvent::ButtonReleased(button) => {
                self.buttons_state.set_released(button);
                let mut events = vec![UserEvent::ButtonReleased(
                    button,
                    self.get_mouse_event(map, screen_size),
                )];

                if (now.duration_since(self.last_pressed_time)).unwrap_or_default() < CLICK_TIMEOUT
                {
                    events.push(UserEvent::Click(
                        button,
                        self.get_mouse_event(map, screen_size),
                    ));

                    if (now.duration_since(self.last_click_time)).unwrap_or_default()
                        < DBL_CLICK_TIMEOUT
                    {
                        events.push(UserEvent::DoubleClick(
                            button,
                            self.get_mouse_event(map, screen_size),
                        ));
                    }

                    self.last_click_time = now;

                    if self.drag_target.take().is_some() {
                        events.push(UserEvent::DragEnded(
                            button,
                            self.get_mouse_event(map, screen_size),
                        ));
                    }
                }

                Some(events)
            }
            RawUserEvent::PointerMoved(position) => {
                let prev_position = self.pointer_position;
                self.pointer_position = position;

                let mut events = vec![UserEvent::PointerMoved(
                    self.get_mouse_event(map, screen_size),
                )];
                if let Some(button) = self.buttons_state.single_pressed() {
                    if self.drag_target.is_none()
                        && position.taxicab_distance(&self.pointer_pressed_position)
                            > DRAG_THRESHOLD
                    {
                        events.push(UserEvent::DragStarted(
                            button,
                            self.get_mouse_event_pos(
                                map,
                                screen_size,
                                self.pointer_pressed_position,
                            ),
                        ));
                    }

                    if self.drag_target.is_some() {
                        events.push(UserEvent::Drag(
                            button,
                            self.get_map_delta(map, screen_size, prev_position),
                            self.get_mouse_event(map, screen_size),
                        ));
                    }
                }

                Some(events)
            }
            RawUserEvent::MouseWheel(delta) => Some(vec![UserEvent::Zoom(
                delta,
                self.get_mouse_event(map, screen_size),
            )]),
            // todo
            RawUserEvent::TouchStart(_touch) => None,
            RawUserEvent::TouchMove(_touch) => None,
            RawUserEvent::TouchEnd(_touch) => None,
        }
    }

    fn get_mouse_event(&self, map: &Map, screen_size: Size) -> MouseEvent {
        self.get_mouse_event_pos(map, screen_size, self.pointer_position)
    }

    fn get_mouse_event_pos(
        &self,
        map: &Map,
        screen_size: Size,
        screen_pointer_position: Point2d,
    ) -> MouseEvent {
        MouseEvent {
            screen_pointer_position,
            map_pointer_position: map.view().px_to_map(self.pointer_position, screen_size),
            buttons: self.buttons_state,
        }
    }

    fn get_map_delta(&self, map: &Map, screen_size: Size, prev_position: Point2d) -> Vec2d<f64> {
        let curr_position = map.view().px_to_map(self.pointer_position, screen_size);
        let prev_position = map.view().px_to_map(prev_position, screen_size);

        curr_position - prev_position
    }
}
