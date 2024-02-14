//! [Layers](Layer) specify a data source and the way the data should be rendered to the map.

use crate::messenger::Messenger;
use crate::render::Canvas;
use crate::view::MapView;
use maybe_sync::{MaybeSend, MaybeSync};
use std::any::Any;
use std::sync::{Arc, RwLock};

pub mod data_provider;
pub mod feature_layer;
mod raster_tile_layer;
pub mod vector_tile_layer;

pub use feature_layer::FeatureLayer;
pub use raster_tile_layer::RasterTileLayer;
pub use vector_tile_layer::VectorTileLayer;

/// Layers specify a data source and the way the data should be rendered to the map.
///
/// There are currently 3 types of layers:
/// * [`RasterTileLayer`] - downloads prerendered tiles from an Internet source and draws them as is.
/// * [`VectorTileLayer`] - downloads vector tiles (in MVT format) from an Internet source and draws them using the
///   provided stylesheet.
/// * [`FeatureLayer`] - draws custom set of geographic objects with the given [`feature_layer::Symbol`];
pub trait Layer: MaybeSend + MaybeSync {
    /// Renders the layer to the given canvas.
    fn render(&self, view: &MapView, canvas: &mut dyn Canvas);
    /// Prepares the layer for rendering with the given `view`. The preparation may include data downloading, decoding
    /// or other asynchronous operations which cannot be awaited for during render cycle..
    fn prepare(&self, view: &MapView);
    /// Sets the messenger for the layer. Messenger is used to notify the application when the layer thinks it should
    /// be updated on the screen.
    fn set_messenger(&mut self, messenger: Box<dyn Messenger>);
    /// A map stores layers as trait objects. This method can be used to convert the trait object into the concrete type.
    fn as_any(&self) -> &dyn Any;
    /// A map stores layers as trait objects. This method can be used to convert the trait object into the concrete type.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Layer + 'static> Layer for Arc<RwLock<T>> {
    fn render(&self, position: &MapView, canvas: &mut dyn Canvas) {
        self.read()
            .expect("lock is poisoned")
            .render(position, canvas)
    }

    fn prepare(&self, view: &MapView) {
        self.read().expect("lock is poisoned").prepare(view)
    }

    fn set_messenger(&mut self, messenger: Box<dyn Messenger>) {
        self.write()
            .expect("lock is poisoned")
            .set_messenger(messenger)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Used for doc-tests
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
