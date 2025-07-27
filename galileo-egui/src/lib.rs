//! Integration between [Galileo](https://docs.rs/galileo/latest/galileo/) and
//! [EGUI](https://docs.rs/galileo/latest/egui).
//!
//! This crate provides a widget [`EguiMap`] for `egui` to render a Galileo map into egui
//! application.
//!
//! With `init` feature you else get an [`InitBuilder`] that can help you set up a simple
//! application with a map. This struct is mainly meant to be used in development environments or
//! for simple examples.

mod egui_map;
pub use egui_map::{EguiMap, EguiMapState};

#[cfg(feature = "init")]
mod init;
#[cfg(feature = "init")]
pub use init::EguiMapOptions;
#[cfg(feature = "init")]
pub use init::InitBuilder;
