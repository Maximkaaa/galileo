//! [Layers](Layer) specify a data source and the way the data should be rendered to the map.

use std::any::Any;
use std::sync::Arc;

use maybe_sync::{MaybeSend, MaybeSync};
use parking_lot::RwLock;

use crate::layer::attribution::Attribution;
use crate::messenger::Messenger;
use crate::render::Canvas;
use crate::view::MapView;
use crate::TileSchema;

pub mod attribution;
pub mod data_provider;
pub mod feature_layer;
pub mod raster_tile_layer;
pub(crate) mod tiles;
pub mod vector_tile_layer;

pub use feature_layer::{FeatureId, FeatureLayer};
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
    fn prepare(&self, view: &MapView, canvas: &mut dyn Canvas);
    /// Sets the messenger for the layer. Messenger is used to notify the application when the layer thinks it should
    /// be updated on the screen.
    fn set_messenger(&mut self, messenger: Box<dyn Messenger>);
    /// A map stores layers as trait objects. This method can be used to convert the trait object into the concrete type.
    fn as_any(&self) -> &dyn Any;
    /// A map stores layers as trait objects. This method can be used to convert the trait object into the concrete type.
    fn as_any_mut(&mut self) -> &mut dyn Any;
    /// Tile schema of the layer if any.
    fn tile_schema(&self) -> Option<TileSchema> {
        None
    }
    /// Returns the attribution of the layer, if available.
    fn attribution(&self) -> Option<Attribution>;
    /// Loads tiles for the layer.
    fn load_tiles(&self) {}
}

impl<T: Layer + 'static> Layer for Arc<RwLock<T>> {
    fn render(&self, position: &MapView, canvas: &mut dyn Canvas) {
        self.read().render(position, canvas)
    }

    fn prepare(&self, view: &MapView, canvas: &mut dyn Canvas) {
        self.read().prepare(view, canvas)
    }

    fn set_messenger(&mut self, messenger: Box<dyn Messenger>) {
        self.write().set_messenger(messenger)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn tile_schema(&self) -> Option<TileSchema> {
        self.read().tile_schema()
    }

    fn attribution(&self) -> Option<Attribution> {
        self.read().attribution()
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

    fn prepare(&self, _view: &MapView, _canvas: &mut dyn Canvas) {
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
    fn attribution(&self) -> Option<Attribution> {
        None
    }
}
