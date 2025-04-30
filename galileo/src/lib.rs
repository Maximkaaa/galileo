//! Galileo is a cross-platform map rendering engine. It supports raster and vector layers, custom and flexible styling,
//! working with different coordinate systems and projects.
//!
//! # Quick start
//!
//! **Cargo.toml:**
//! ```toml
//! [package]
//! name = "map-view"
//! version = "0.1.0"
//! edition = "2024"
//!
//! [dependencies]
//! galileo = {git = "https://github.com/maximkaaa/galileo"}
//! galileo-types = {git = "https://github.com/maximkaaa/galileo"}
//! galileo-egui = {git = "https://github.com/maximkaaa/galileo"}
//! ```
//!
//! **src/main.rs:**
//! ```no_run
//! use galileo::layer::raster_tile_layer::RasterTileLayerBuilder;
//! use galileo::{Map, MapBuilder};
//! use galileo::layer::FeatureLayer;
//! use galileo::symbol::CirclePointSymbol;
//! use galileo::galileo_types::latlon;
//! use galileo_types::geo::Crs;
//! use galileo::Color;
//!
//! fn main() {
//!     run()
//! }
//!
//! // Creates the window and starts the egui event loop.
//! pub(crate) fn run() {
//!     galileo_egui::init(create_map(), [])
//!         .expect("Couldn't create window");
//! }
//!
//! // build a map showing the Open Street Map raster data for Seoul, South Korea
//! fn create_map()->Map {
//!     MapBuilder::default()
//!         // Set the position of the center of the map view
//!         .with_latlon(37.566, 126.9784)
//!         // Set the size of the smallest visible feature (zoom level) for the view to start with.
//!         .with_z_level(8)
//!         // Add the Open Street Maps raster tile server as a layer
//!         .with_layer(RasterTileLayerBuilder::new_osm().build().unwrap())
//!         // Add a blue dot at the coordinates specified
//!         .with_layer(FeatureLayer::new(
//!             // the position of the marker we're going to add, in the WGS84 CRS
//!             vec![latlon!(37.566, 126.9784)],
//!             // a blue circle with fixed size of 5.0 pixels
//!             CirclePointSymbol::new(Color::BLUE, 5.0),
//!             // WGS84 is the coordinate reference system that's based on latitude/longitude pairs.
//!             Crs::WGS84,
//!         ))
//!         .build()
//! }
//! ```
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
