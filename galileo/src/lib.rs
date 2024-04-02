//! Galileo is a cross-platform map rendering engine. It supports raster and vector layers, custom and flexible styling,
//! working with different coordinate systems and projects.
//!
//! # Quick start
//!
//! You can create a simple interactive map with two layers by this code:
//!
//! ```no_run
//! use galileo::{MapBuilder, TileSchema };
//! use galileo::layer::FeatureLayer;
//! use galileo::symbol::CirclePointSymbol;
//! use galileo::galileo_types::latlon;
//! use galileo_types::geo::Crs;
//! use galileo::Color;
//!
//! # tokio_test::block_on(async {
//! MapBuilder::new()
//!     .center(latlon!(37.566, 126.9784))
//!     .resolution(TileSchema::web(18).lod_resolution(8).unwrap())
//!     .with_raster_tiles(|index| {
//!         format!(
//!             "https://tile.openstreetmap.org/{}/{}/{}.png",
//!             index.z, index.x, index.y
//!         )},
//!         TileSchema::web(18))
//!     .with_layer(FeatureLayer::new(
//!         vec![latlon!(37.566, 126.9784)],
//!         CirclePointSymbol::new(Color::BLUE, 5.0),
//!         Crs::WGS84,
//!     ))
//!     .build()
//!     .await
//!     .run();
//! # });
//! ```
//!
//! This will show a map with Open Street Maps base and one blue circle in the center of the map. Map builder takes
//! care of creating a window, setting up GPU context and configuring user interactions to control the map position
//! with mouse or touch.
//!
//! Calling [`.run()`](GalileoMap) starts `winit` event loop, which will run until the user
//! closes the window.
//!
//! Running the map in a dedicated window is quite straightforward, but to integrate Galileo map into your application
//! and interact with it you will need some understanding of what happens under the hood of the [`MapBuilder`].
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

#![warn(clippy::unwrap_used)]
#![warn(missing_docs)]

pub(crate) mod async_runtime;
mod color;
pub mod control;
pub mod decoded_image;
pub mod error;
pub mod layer;
mod lod;
mod map;
mod messenger;
mod platform;
pub mod render;
pub mod tile_scheme;
mod view;

#[cfg(feature = "winit")]
pub mod winit;

#[cfg(all(feature = "winit", feature = "wgpu"))]
mod galileo_map;
#[cfg(all(feature = "winit", feature = "wgpu"))]
pub use galileo_map::{GalileoMap, MapBuilder};

pub use color::Color;
pub use layer::feature_layer::symbol;
pub use lod::Lod;
pub use map::{LayerCollection, Map};
pub use messenger::{DummyMessenger, Messenger};
pub use tile_scheme::TileSchema;
pub use view::MapView;

// Reexport galileo_types
pub use galileo_types;
