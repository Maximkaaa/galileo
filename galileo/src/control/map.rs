use std::time::Duration;

use nalgebra::Vector2;

use crate::control::{EventPropagation, MouseButton, UserEvent, UserEventHandler};
use crate::map::Map;
use crate::view::MapView;

const DEFAULT_ZOOM_DURATION: Duration = Duration::from_millis(50);

/// Event handler of a map, providing panning, zooming and tilting capabilities.
#[derive(Default)]
pub struct MapController {
    parameters: MapControllerParameters,
}

pub struct MapControllerParameters {
    zoom_duration: Duration,
    zoom_speed: f64,
    min_resolution: f64,
    max_resolution: f64,

    rotation_speed: f64,
    max_rotation_x: f64,
}

impl Default for MapControllerParameters {
    fn default() -> Self {
        Self {
            zoom_duration: DEFAULT_ZOOM_DURATION,
            zoom_speed: 0.2,
            max_resolution: 156543.03392800014 / 8.0,
            min_resolution: 156543.03392800014 / 8.0 / 2.0f64.powi(16),
            rotation_speed: 0.005,
            max_rotation_x: 80f64.to_radians(),
        }
    }
}

impl UserEventHandler for MapController {
    fn handle(&self, event: &UserEvent, map: &mut Map) -> EventPropagation {
        match event {
            UserEvent::DragStarted(button, _)
                if *button == MouseButton::Left
                    || *button == MouseButton::Right
                    || *button == MouseButton::Other =>
            {
                EventPropagation::Consume
            }
            UserEvent::Drag(button, delta, e) => match button {
                MouseButton::Left | MouseButton::Other => {
                    let current_position = e.screen_pointer_position;
                    let prev_position = current_position - delta;

                    map.set_view(
                        map.view()
                            .translate_by_pixels(prev_position, current_position),
                    );
                    EventPropagation::Stop
                }
                MouseButton::Right => {
                    map.set_view(self.get_rotation(map.view(), *delta));
                    EventPropagation::Stop
                }
                _ => EventPropagation::Propagate,
            },
            UserEvent::Scroll(delta, mouse_event) => {
                let zoom = self.get_zoom(*delta, map.view().resolution());
                let target = map
                    .target_view()
                    .zoom(zoom, mouse_event.screen_pointer_position);
                map.animate_to(target, self.parameters.zoom_duration);

                EventPropagation::Stop
            }
            UserEvent::Zoom(zoom, center) => {
                let target = map.view().zoom(*zoom, *center);
                map.set_view(target);

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

    fn get_rotation(&self, curr_view: &MapView, px_delta: Vector2<f64>) -> MapView {
        let dz = px_delta.x * self.parameters.rotation_speed;

        let rotation_z = curr_view.rotation_z() + dz;
        let mut rotation_x = curr_view.rotation_x() - px_delta.y * self.parameters.rotation_speed;

        if rotation_x < 0.0 {
            rotation_x = 0.0;
        } else if rotation_x > self.parameters.max_rotation_x {
            rotation_x = self.parameters.max_rotation_x;
        };

        curr_view.with_rotation(rotation_x, rotation_z)
    }
}
