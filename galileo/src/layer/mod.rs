use crate::messenger::Messenger;
use crate::render::{Canvas, Renderer};
use crate::view::MapView;
use maybe_sync::{MaybeSend, MaybeSync};
use std::any::Any;
use std::sync::{Arc, RwLock};

pub mod feature;
pub mod raster_tile;
pub mod tile_provider;
pub mod vector_tile;

pub trait Layer: MaybeSend + MaybeSync {
    fn render<'a>(&self, position: &MapView, canvas: &'a mut dyn Canvas);
    fn prepare(&self, view: &MapView, renderer: &Arc<RwLock<dyn Renderer>>);
    fn set_messenger(&self, messenger: Box<dyn Messenger>);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Layer> Layer for Arc<RwLock<T>> {
    fn render<'a>(&self, position: &MapView, canvas: &'a mut dyn Canvas) {
        self.read().unwrap().render(position, canvas)
    }

    fn prepare(&self, view: &MapView, renderer: &Arc<RwLock<dyn Renderer>>) {
        self.read().unwrap().prepare(view, renderer)
    }

    fn set_messenger(&self, messenger: Box<dyn Messenger>) {
        self.read().unwrap().set_messenger(messenger)
    }

    fn as_any(&self) -> &dyn Any {
        todo!()
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        todo!()
    }
}
