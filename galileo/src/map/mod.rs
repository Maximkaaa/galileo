use crate::layer::Layer;
use crate::messenger::Messenger;
use crate::render::Renderer;
use crate::view::MapView;
use galileo_types::size::Size;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use web_time::SystemTime;

const FRAME_DURATION: Duration = Duration::from_millis(16);

pub struct Map {
    view: MapView,
    layers: Vec<Box<dyn Layer>>,
    messenger: Box<dyn Messenger>,
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
        messenger: impl Messenger + 'static,
    ) -> Self {
        Self {
            view,
            layers,
            messenger: Box::new(messenger),
            animation: None,
        }
    }

    pub fn view(&self) -> MapView {
        self.view
    }

    pub fn layers(&self) -> &[Box<dyn Layer>] {
        &self.layers
    }

    pub fn layer_mut(&mut self, index: usize) -> Option<&mut Box<dyn Layer>> {
        self.layers.get_mut(index)
    }

    pub(crate) fn set_view(&mut self, view: MapView) {
        self.view = view;
        self.messenger.request_redraw();
    }

    pub fn load_layers(&self, renderer: &Arc<RwLock<dyn Renderer>>) {
        for layer in &self.layers {
            layer.prepare(self.view, renderer);
        }
    }

    pub fn redraw(&self) {
        self.messenger.request_redraw()
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
            self.view = animation.end_view;
            self.animation = None;
        } else {
            self.view = animation.start_view.interpolate(animation.end_view, k);
        }

        self.redraw();
    }

    pub fn target_view(&self) -> MapView {
        self.animation
            .as_ref()
            .map(|v| v.end_view)
            .unwrap_or(self.view)
    }

    pub fn animate_to(&mut self, target: MapView, duration: Duration) {
        self.animation = Some(AnimationParameters {
            start_view: self.view,
            end_view: target,
            start_time: SystemTime::now() - FRAME_DURATION,
            duration,
        });
    }

    pub fn set_size(&mut self, new_size: Size) {
        self.view = self.view.with_size(new_size);
    }
}
