use crate::layer::Layer;
use crate::map::layer_collection::LayerCollection;
use crate::messenger::Messenger;
use crate::view::MapView;
use galileo_types::cartesian::Size;
use std::time::Duration;
use web_time::SystemTime;

pub mod layer_collection;

const FRAME_DURATION: Duration = Duration::from_millis(16);

pub struct Map {
    view: MapView,
    layers: LayerCollection,
    messenger: Option<Box<dyn Messenger>>,
    animation: Option<AnimationParameters>,
}

struct AnimationParameters {
    start_view: MapView,
    end_view: MapView,
    start_time: SystemTime,
    duration: Duration,
}

impl Map {
    pub fn new(
        view: MapView,
        layers: Vec<Box<dyn Layer>>,
        messenger: Option<impl Messenger + 'static>,
    ) -> Self {
        let messenger: Option<Box<dyn Messenger>> = if let Some(m) = messenger {
            Some(Box::new(m))
        } else {
            None
        };
        Self {
            view,
            layers: layers.into(),
            messenger,
            animation: None,
        }
    }

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

    pub(crate) fn set_view(&mut self, view: MapView) {
        self.view = view;
        if let Some(messenger) = &self.messenger {
            messenger.request_redraw();
        }
    }

    pub fn load_layers(&self) {
        for layer in self.layers.iter_visible() {
            layer.prepare(&self.view);
        }
    }

    pub fn redraw(&self) {
        if let Some(messenger) = &self.messenger {
            messenger.request_redraw()
        }
    }

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

    pub fn target_view(&self) -> &MapView {
        self.animation
            .as_ref()
            .map(|v| &v.end_view)
            .unwrap_or(&self.view)
    }

    pub fn animate_to(&mut self, target: MapView, duration: Duration) {
        self.animation = Some(AnimationParameters {
            start_view: self.view.clone(),
            end_view: target,
            start_time: SystemTime::now() - FRAME_DURATION,
            duration,
        });
    }

    pub fn set_size(&mut self, new_size: Size) {
        self.view = self.view.with_size(new_size);
    }
}
