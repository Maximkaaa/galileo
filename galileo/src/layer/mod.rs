use crate::messenger::Messenger;
use crate::render::{Canvas, Renderer};
use crate::view::MapView;
use maybe_sync::{MaybeSend, MaybeSync};
use std::sync::{Arc, RwLock};

pub mod feature_layer;
pub mod raster_tile;
pub mod tile_provider;
pub mod vector_tile_layer;

pub use feature_layer::FeatureLayer;
pub use raster_tile::RasterTileLayer;
pub use vector_tile_layer::VectorTileLayer;

pub trait Layer: MaybeSend + MaybeSync {
    fn render(&self, view: &MapView, canvas: &mut dyn Canvas);
    fn prepare(&self, view: &MapView, renderer: &Arc<RwLock<dyn Renderer>>);
    fn set_messenger(&self, messenger: Box<dyn Messenger>);
}

impl<T: Layer> Layer for Arc<RwLock<T>> {
    fn render(&self, position: &MapView, canvas: &mut dyn Canvas) {
        self.read().unwrap().render(position, canvas)
    }

    fn prepare(&self, view: &MapView, renderer: &Arc<RwLock<dyn Renderer>>) {
        self.read().unwrap().prepare(view, renderer)
    }

    fn set_messenger(&self, messenger: Box<dyn Messenger>) {
        self.read().unwrap().set_messenger(messenger)
    }
}
