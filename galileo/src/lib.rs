//! Galileo is a cross-platform map rendering engine. It supports raster and vector layers, custom and flexible styling,
//! working with different coordinate systems and projects.
//!
//! # Quick start
//!
//! TODO
//!
//! # Main components of Galileo
//!
//! As surprising as it is, everything in a mapping library revolves around
//!
//! * [`Map`] struct, which is quite simple by itself and contains only currently displayed
//!   [`MapView`], inner state, such as animation parameters, and a set of
//! * [`layers`](layer) that actually contain data and know how it should be displayed. There are different
//!   types of layers depending on what kind of data they use (images, vector tiles, geometric features etc) and on
//!   their capabilities for transforming that data into what a user wants to see. To render the data layers use
//! * [`renderer`](render), which is responsible for converting primitives into the images the user sees.
//!
//! As you have probably noticed nothing of the above deals with user interactions or events. You can think of the map
//! (with its layers) as a map you hang on your wall. It just shows some geo-data and does nothing else. So if you
//! create a console utility, server or some kind of presentation application, these three would be all you need.
//!
//! In case a user is supposed to interact with the map in your application, you would also need
//!
//! * [`EventProcessor`](control::EventProcessor) to convert raw system event into
//!   some intermediate representation, more convenient to deal with, and some
//! * [`controls`](control) that actually change state of the map or layers based on the user input.

pub(crate) mod async_runtime;
pub mod attribution;
mod color;
pub mod control;
pub mod decoded_image;
pub mod error;
pub mod layer;
mod lod;
mod map;
mod messenger;
pub mod platform;
pub mod render;
pub mod tile_schema;
mod view;

#[cfg(test)]
pub(crate) mod tests;

#[cfg(feature = "winit")]
pub mod winit;

pub use color::Color;
// Reexport galileo_types
pub use galileo_types;
pub use layer::feature_layer::symbol;
pub use lod::Lod;
pub use map::{LayerCollection, Map, MapBuilder};
pub use messenger::{DummyMessenger, Messenger};
pub use tile_schema::TileSchema;
pub use view::MapView;
