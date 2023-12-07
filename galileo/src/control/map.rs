use crate::control::{EventPropagation, MouseButton, UserEvent, UserEventHandler};
use crate::map::Map;
use std::time::Duration;

const DEFAULT_ZOOM_DURATION: Duration = Duration::from_millis(50);

#[derive(Default)]
pub struct MapController {
    parameters: MapControllerParameters,
}

pub struct MapControllerParameters {
    zoom_duration: Duration,
    zoom_speed: f64,
    min_resolution: f64,
    max_resolution: f64,
}

impl Default for MapControllerParameters {
    fn default() -> Self {
        Self {
            zoom_duration: DEFAULT_ZOOM_DURATION,
            zoom_speed: 0.2,
            max_resolution: 156543.03392800014 / 8.0,
            min_resolution: 156543.03392800014 / 8.0 / 2.0f64.powi(16),
        }
    }
}

impl UserEventHandler for MapController {
    fn handle(&self, event: &UserEvent, map: &mut Map) -> EventPropagation {
        match event {
            UserEvent::DragStarted(button, _) if *button == MouseButton::Left => {
                EventPropagation::Consume
            }
            UserEvent::Drag(_, delta, _) => {
                map.set_view(map.view().translate(*delta));
                EventPropagation::Stop
            }
            UserEvent::Zoom(delta, mouse_event) => {
                let zoom = self.get_zoom(*delta, map.view().resolution);
                let target = map
                    .target_view()
                    .zoom(zoom, mouse_event.map_pointer_position);
                map.animate_to(target, self.parameters.zoom_duration);

                EventPropagation::Stop
            }
            _ => EventPropagation::Propagate,
        }
    }
}

impl MapController {
    fn get_zoom(&self, delta: f64, current_resolution: f64) -> f64 {
        let zoom = (self.parameters.zoom_speed + 1.0).powf(-delta);
        let target_resolution = current_resolution * zoom;
        if target_resolution > self.parameters.max_resolution {
            self.parameters.max_resolution / current_resolution
        } else if target_resolution < self.parameters.min_resolution {
            self.parameters.min_resolution / current_resolution
        } else {
            zoom
        }
    }
}
