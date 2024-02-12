use crate::messenger::Messenger;
use crate::render::Canvas;
use crate::view::MapView;
use maybe_sync::{MaybeSend, MaybeSync};
use std::any::Any;
use std::sync::{Arc, RwLock};

pub mod data_provider;
pub mod feature_layer;
pub mod raster_tile_layer;
pub mod vector_tile_layer;

pub use feature_layer::FeatureLayer;
pub use raster_tile_layer::RasterTileLayer;
pub use vector_tile_layer::VectorTileLayer;

pub trait Layer: MaybeSend + MaybeSync {
    fn render(&self, view: &MapView, canvas: &mut dyn Canvas);
    fn prepare(&self, view: &MapView);
    fn set_messenger(&mut self, messenger: Box<dyn Messenger>);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Layer + 'static> Layer for Arc<RwLock<T>> {
    fn render(&self, position: &MapView, canvas: &mut dyn Canvas) {
        self.read().unwrap().render(position, canvas)
    }

    fn prepare(&self, view: &MapView) {
        self.read().unwrap().prepare(view)
    }

    fn set_messenger(&mut self, messenger: Box<dyn Messenger>) {
        self.write().unwrap().set_messenger(messenger)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(feature = "_tests")]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TestLayer(pub &'static str);

#[cfg(feature = "_tests")]
impl Layer for TestLayer {
    fn render(&self, _view: &MapView, _canvas: &mut dyn Canvas) {
        unimplemented!()
    }

    fn prepare(&self, _view: &MapView) {
        unimplemented!()
    }

    fn set_messenger(&mut self, _messenger: Box<dyn Messenger>) {
        unimplemented!()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
