use std::time::Duration;

use galileo_types::cartesian::Vector2;

use crate::control::{EventPropagation, MouseButton, UserEvent, UserEventHandler};
use crate::map::Map;
use crate::view::MapView;

const DEFAULT_ZOOM_DURATION: Duration = Duration::from_millis(50);
const ROTATION_SPEED_K: f64 = 0.005;

/// Configuration of a [`MapController`]
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct MapControllerConfiguration {
    zoom_duration: Duration,
    zoom_speed: f64,
    min_resolution: f64,
    max_resolution: f64,

    rotation_speed: f64,
    min_rotation_x: f64,
    max_rotation_x: f64,
    min_rotation_z: f64,
    max_rotation_z: f64,
}

impl Default for MapControllerConfiguration {
    fn default() -> Self {
        Self {
            zoom_duration: DEFAULT_ZOOM_DURATION,
            zoom_speed: 0.2,
            max_resolution: 156543.03392800014 / 8.0,
            min_resolution: 156543.03392800014 / 8.0 / 2.0f64.powi(16),
            rotation_speed: 1.0,
            min_rotation_x: 0f64,
            max_rotation_x: 80f64.to_radians(),
            min_rotation_z: f64::MIN,
            max_rotation_z: f64::MAX,
        }
    }
}

impl MapControllerConfiguration {
    /// Duration of the zoom animation when mouse wheel is turned.
    pub fn zoom_duration(&self) -> Duration {
        self.zoom_duration
    }

    /// Sets duration of the zoom animation when mouse wheel is turned.
    pub fn with_zoom_duration(mut self, duration: Duration) -> Self {
        self.zoom_duration = duration;
        self
    }

    /// Sets duration of the zoom animation when mouse wheel is turned.
    pub fn set_zoom_duration(&mut self, duration: Duration) {
        self.zoom_duration = duration;
    }

    /// Magnitude of the zoom on every mouse wheel turn.
    ///
    /// For example, the value of `0.2` means, that every time the mouse wheel is turned, the map
    /// will be zoomed by 0.2 times.
    pub fn zoom_apeed(&self) -> f64 {
        self.zoom_speed
    }

    /// Sets magnitude of the zoom on every mouse wheel turn.
    ///
    /// For example, the value of `0.2` means, that every time the mouse wheel is turned, the map
    /// will be zoomed by 0.2 times.
    pub fn with_zoom_speed(mut self, speed: f64) -> Self {
        self.zoom_speed = speed;
        self
    }

    /// Sets magnitude of the zoom on every mouse wheel turn.
    ///
    /// For example, the value of `0.2` means, that every time the mouse wheel is turned, the map
    /// will be zoomed by 0.2 times.
    pub fn set_zoom_speed(&mut self, speed: f64) {
        self.zoom_speed = speed;
    }

    /// Maximum allowed resolution.
    pub fn max_resolution(&self) -> f64 {
        self.max_resolution
    }

    /// Sets maximum allowed resolution.
    pub fn with_max_resolution(mut self, resolution: f64) -> Self {
        self.max_resolution = resolution;
        self
    }

    /// Sets maximum allowed resolution.
    pub fn set_max_resolution(&mut self, resolution: f64) {
        self.max_resolution = resolution;
    }

    /// Minimum allowed resolution.
    pub fn min_resolution(&self) -> f64 {
        self.min_resolution
    }

    /// Sets minimum allowed resolution.
    pub fn with_min_resolution(mut self, resolution: f64) -> Self {
        self.min_resolution = resolution;
        self
    }

    /// Sets minimum allowed resolution.
    pub fn set_min_resolution(&mut self, resolution: f64) {
        self.max_resolution = resolution;
    }

    /// Sensitivity for map rotation by dragging right mouse button.
    ///
    /// The value here is an abstract multiplier. Default value is `1.0`. Use higher values for
    /// higher sensitivity. Negative value will inverse rotation direction.
    pub fn rotation_speed(&self) -> f64 {
        self.rotation_speed
    }

    /// Sets sensitivity for map rotation by dragging right mouse button.
    ///
    /// The value here is an abstract multiplier. Default value is `1.0`. Use higher values for
    /// higher sensitivity. Negative value will inverse rotation direction.
    pub fn with_rotation_speed(mut self, speed: f64) -> Self {
        self.rotation_speed = speed;
        self
    }

    /// Sets sensitivity for map rotation by dragging right mouse button.
    ///
    /// The value here is an abstract multiplier. Default value is `1.0`. Use higher values for
    /// higher sensitivity. Negative value will inverse rotation direction.
    pub fn set_rotation_speed(&mut self, speed: f64) {
        self.rotation_speed = speed;
    }

    /// Minimum allowed tilt of the map in radians.
    ///
    /// The value of `0.0` means the map is viewd from above. The value of `PI/2` corresponds to
    /// the map tilted horizontally.
    pub fn min_rotation_x(&self) -> f64 {
        self.min_rotation_x
    }

    /// Sets minimum allowed tilt of the map in radians.
    ///
    /// The value of `0.0` means the map is viewd from above. The value of `PI/2` corresponds to
    /// the map tilted horizontally.
    pub fn with_min_rotation_x(mut self, rotation: f64) -> Self {
        self.min_rotation_x = rotation;
        self
    }

    /// Sets minimum allowed tilt of the map in radians.
    ///
    /// The value of `0.0` means the map is viewd from above. The value of `PI/2` corresponds to
    /// the map tilted horizontally.
    pub fn set_min_rotation_x(&mut self, rotation: f64) {
        self.min_rotation_x = rotation;
    }

    /// Maximum allowed tilt of the map in radians.
    ///
    /// The value of `0.0` means the map is viewd from above. The value of `PI/2` corresponds to
    /// the map tilted horizontally.
    pub fn max_rotation_x(&self) -> f64 {
        self.max_rotation_x
    }

    /// Sets maximum allowed tilt of the map in radians.
    ///
    /// The value of `0.0` means the map is viewd from above. The value of `PI/2` corresponds to
    /// the map tilted horizontally.
    pub fn with_max_rotation_x(mut self, rotation: f64) -> Self {
        self.max_rotation_x = rotation;
        self
    }

    /// Sets maximum allowed tilt of the map in radians.
    ///
    /// The value of `0.0` means the map is viewd from above. The value of `PI/2` corresponds to
    /// the map tilted horizontally.
    pub fn set_max_rotation_x(&mut self, rotation: f64) {
        self.min_rotation_x = rotation;
    }

    /// Minimum allowed rotation of the map around in radians.
    ///
    /// Positive values correspond to counterclockwise rotation.
    pub fn min_rotation_z(&self) -> f64 {
        self.min_rotation_z
    }

    /// Sets minimum allowed rotation of the map around in radians.
    ///
    /// Positive values correspond to counterclockwise rotation.
    pub fn with_min_rotation_z(mut self, rotation: f64) -> Self {
        self.min_rotation_z = rotation;
        self
    }

    /// Sets minimum allowed rotation of the map around in radians.
    ///
    /// Positive values correspond to counterclockwise rotation.
    pub fn set_min_rotation_z(&mut self, rotation: f64) {
        self.min_rotation_z = rotation;
    }

    /// Maximum allowed rotation of the map around in radians.
    ///
    /// Positive values correspond to counterclockwise rotation.
    pub fn max_rotation_z(&self) -> f64 {
        self.max_rotation_z
    }

    /// Sets maximum allowed rotation of the map around in radians.
    ///
    /// Positive values correspond to counterclockwise rotation.
    pub fn with_max_rotation_z(mut self, rotation: f64) -> Self {
        self.max_rotation_z = rotation;
        self
    }

    /// Sets maximum allowed rotation of the map around in radians.
    ///
    /// Positive values correspond to counterclockwise rotation.
    pub fn set_max_rotation_z(&mut self, rotation: f64) {
        self.min_rotation_z = rotation;
    }

    /// Disables tilting of the map by setting min and max rotation x to `0.0.
    pub fn with_disable_rotation_x(mut self) -> Self {
        self.min_rotation_x = 0.0;
        self.max_rotation_x = 0.0;
        self
    }

    /// Disables rotation of the map by setting min and max rotation z to `0.0.
    pub fn with_disable_rotation_z(mut self) -> Self {
        self.min_rotation_z = 0.0;
        self.max_rotation_z = 0.0;
        self
    }
}

/// Event handler of a map, providing panning, zooming and tilting capabilities.
#[derive(Default, Copy, Clone, PartialEq, Debug)]
pub struct MapController {
    config: MapControllerConfiguration,
}

impl MapController {
    /// Creates a new instance of `MapController` with the given configuration.
    pub fn new(config: MapControllerConfiguration) -> Self {
        Self { config }
    }

    /// Returns the current configuration of the controller.
    pub fn config(&self) -> MapControllerConfiguration {
        self.config
    }

    /// Update the configuration of the controller.
    pub fn set_config(&mut self, config: MapControllerConfiguration) {
        self.config = config;
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
                    let prev_position = current_position - *delta;

                    let target = map
                        .view()
                        .translate_by_pixels(prev_position, current_position);

                    let adjusted = self.adjust_target_view(target);

                    map.set_view(adjusted);
                    EventPropagation::Stop
                }
                MouseButton::Right => {
                    let target = self.get_rotation(map.view(), *delta);
                    let adjusted = self.adjust_target_view(target);
                    map.set_view(adjusted);

                    EventPropagation::Stop
                }
                _ => EventPropagation::Propagate,
            },
            UserEvent::Scroll(delta, mouse_event) => {
                let zoom = self.get_zoom(*delta);
                let target = map
                    .target_view()
                    .zoom(zoom, mouse_event.screen_pointer_position);
                let adjusted = self.adjust_target_view(target);
                map.animate_to(adjusted, self.config.zoom_duration);

                EventPropagation::Stop
            }
            UserEvent::Zoom(zoom, center) => {
                let target = map.view().zoom(*zoom, *center);
                let adjusted = self.adjust_target_view(target);
                map.set_view(adjusted);

                EventPropagation::Stop
            }
            _ => EventPropagation::Propagate,
        }
    }
}

impl MapController {
    fn get_zoom(&self, delta: f64) -> f64 {
        (self.config.zoom_speed + 1.0).powf(-delta)
    }

    fn get_rotation(&self, curr_view: &MapView, px_delta: Vector2) -> MapView {
        let dz = px_delta.dx() * self.config.rotation_speed * ROTATION_SPEED_K;

        let rotation_z = curr_view.rotation_z() + dz;
        let rotation_x =
            curr_view.rotation_x() - px_delta.dy() * self.config.rotation_speed * ROTATION_SPEED_K;

        curr_view.with_rotation(rotation_x, rotation_z)
    }

    /// Adjusts target view according to the controller configuration.
    fn adjust_target_view(&self, mut target: MapView) -> MapView {
        if target.resolution() < self.config.min_resolution {
            target = target.with_resolution(self.config.min_resolution);
        }

        if target.resolution() > self.config.max_resolution {
            target = target.with_resolution(self.config.max_resolution);
        }

        if target.rotation_x() > self.config.max_rotation_x {
            target = target.with_rotation_x(self.config.max_rotation_x);
        }

        if target.rotation_x() < self.config.min_rotation_x {
            target = target.with_rotation_x(self.config.min_rotation_x);
        }

        if target.rotation_z() > self.config.max_rotation_z {
            target = target.with_rotation_z(self.config.max_rotation_z);
        }

        if target.rotation_z() < self.config.min_rotation_z {
            target = target.with_rotation_z(self.config.min_rotation_z);
        }

        target
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use galileo_types::latlon;

    use super::*;

    #[test]
    fn min_resolution_is_adjusted() {
        let mut controller = MapController::default();
        let target = MapView::new(&latlon!(0.0, 0.0), controller.config.min_resolution / 2.0);
        let adjusted = controller.adjust_target_view(target.clone());

        assert_relative_eq!(adjusted.resolution(), controller.config.min_resolution);

        controller.config.min_resolution = 10.0;
        let target = target.with_resolution(1.0);
        let adjusted = controller.adjust_target_view(target);
        assert_relative_eq!(adjusted.resolution(), controller.config.min_resolution);
    }

    #[test]
    fn max_resolution_is_adjusted() {
        let mut controller = MapController::default();
        let target = MapView::new(&latlon!(0.0, 0.0), controller.config.max_resolution * 2.0);
        let adjusted = controller.adjust_target_view(target.clone());

        assert_relative_eq!(adjusted.resolution(), controller.config.max_resolution);

        controller.config.max_resolution = 10.0;
        let target = target.with_resolution(100.0);
        let adjusted = controller.adjust_target_view(target);
        assert_relative_eq!(adjusted.resolution(), controller.config.max_resolution);
    }

    #[test]
    fn rotation_x_is_adjusted() {
        let mut controller = MapController::default();
        controller.config.min_rotation_x = 10f64.to_radians();

        let target = MapView::new(&latlon!(0.0, 0.0), controller.config.max_resolution)
            .with_rotation_x(5f64.to_radians());
        let adjusted = controller.adjust_target_view(target.clone());

        assert_relative_eq!(adjusted.rotation_x(), 10f64.to_radians());

        controller.config.max_rotation_x = 50f64.to_radians();

        let target = target.with_rotation_x(55f64.to_radians());
        let adjusted = controller.adjust_target_view(target.clone());

        assert_relative_eq!(adjusted.rotation_x(), 50f64.to_radians());
    }

    #[test]
    fn rotation_y_is_adjusted() {
        let mut controller = MapController::default();
        controller.config.min_rotation_z = -10f64.to_radians();

        let target = MapView::new(&latlon!(0.0, 0.0), controller.config.max_resolution)
            .with_rotation_z(-15f64.to_radians());
        let adjusted = controller.adjust_target_view(target.clone());

        assert_relative_eq!(adjusted.rotation_z(), -10f64.to_radians());

        controller.config.max_rotation_z = 50f64.to_radians();

        let target = target.with_rotation_z(55f64.to_radians());
        let adjusted = controller.adjust_target_view(target.clone());

        assert_relative_eq!(adjusted.rotation_z(), 50f64.to_radians());
    }
}
