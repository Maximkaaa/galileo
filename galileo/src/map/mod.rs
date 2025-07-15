use std::time::Duration;

use galileo_types::cartesian::Size;
use web_time::SystemTime;

use crate::layer::Layer;
use crate::messenger::Messenger;
use crate::view::MapView;

mod builder;
mod layer_collection;

pub use builder::MapBuilder;
pub use layer_collection::LayerCollection;

const FRAME_DURATION: Duration = Duration::from_millis(16);

/// Map specifies a set of layers, and the view that should be rendered.
pub struct Map {
    view: MapView,
    layers: LayerCollection,
    messenger: Option<Box<dyn Messenger>>,
    animation: Option<AnimationParameters>,
    /// DPI scale factor. Is used to adjust sizes of rendered features to be correctly displayed on HiDPI screens.
    pub dpi_scale_factor: f32,
}

struct AnimationParameters {
    start_view: MapView,
    end_view: MapView,
    start_time: SystemTime,
    duration: Duration,
}

impl Map {
    /// Creates a new map.
    pub fn new(
        view: MapView,
        layers: Vec<Box<dyn Layer>>,
        messenger: Option<Box<dyn Messenger + 'static>>,
    ) -> Self {
        Self {
            view,
            layers: layers.into(),
            messenger,
            animation: None,
            dpi_scale_factor: 1.0,
        }
    }

    /// Current view of the map.
    pub fn view(&self) -> &MapView {
        &self.view
    }

    /// Returns the list of map's layers.
    pub fn layers(&self) -> &LayerCollection {
        &self.layers
    }

    /// Returns a mutable reference to the list of map's layers.
    pub fn layers_mut(&mut self) -> &mut LayerCollection {
        &mut self.layers
    }

    /// Changes the view of the map to the given one.
    pub fn set_view(&mut self, view: MapView) {
        self.view = view;
        if let Some(messenger) = &self.messenger {
            messenger.request_redraw();
        }
    }

    /// Request redraw of the map.
    pub fn redraw(&self) {
        if let Some(messenger) = &self.messenger {
            messenger.request_redraw()
        }
    }

    /// Update the view of the map before the rendering in case [`Map::animate_to`] was called.
    pub fn animate(&mut self) {
        let Some(animation) = &self.animation else {
            return;
        };

        let now = SystemTime::now();
        let k = now
            .duration_since(animation.start_time)
            .unwrap_or_default()
            .as_millis() as f64
            / animation.duration.as_millis() as f64;

        if k >= 1.0 {
            let animation = self
                .animation
                .take()
                .expect("the value was removed unexpectedly");
            self.view = animation.end_view;
        } else {
            self.view = animation.start_view.interpolate(&animation.end_view, k);
        }

        self.redraw();
    }

    /// Target view of the current animation.
    pub fn target_view(&self) -> &MapView {
        self.animation
            .as_ref()
            .map(|v| &v.end_view)
            .unwrap_or(&self.view)
    }

    /// Request a gradual change of the map view to the specified view.
    pub fn animate_to(&mut self, target: MapView, duration: Duration) {
        self.animation = Some(AnimationParameters {
            start_view: self.view.clone(),
            end_view: target,
            start_time: SystemTime::now() - FRAME_DURATION,
            duration,
        });
    }

    /// Set the size of the map.
    pub fn set_size(&mut self, new_size: Size) {
        self.view = self.view.with_size(new_size);
    }

    /// Sets the new event messenger for the map.
    pub fn set_messenger(&mut self, messenger: Option<impl Messenger + 'static>) {
        let messenger: Option<Box<dyn Messenger>> = if let Some(m) = messenger {
            Some(Box::new(m))
        } else {
            None
        };

        self.messenger = messenger;
    }
}
