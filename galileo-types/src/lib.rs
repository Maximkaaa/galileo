//! This crate contains a set of geometric primitives and operations on them used in GIS systems. It includes
//! geometries themselves, projections and coordinate systems.
//!
//! The approach taken by this crate is trait-first approach. All business logic and operations are defined in traits
//! and some simple default implementations are provided for convenience. The traits are designed to be simple to
//! implement.
//!
//! # Projected vs geographic coordinates
//!
//! GIS systems work with geometries in two representations:
//! * geographic coordinates are defined by *latitude* and *longitude*
//! * projected coordinates are defined in cartesian *X* and *Y* on a flat surface of the Earth
//!
//! Most GIS systems do not distinguish these coordinates and just consider *latitude* to be *Y* and *longitude* to be
//! *X*. This brings a lot of confusion, starting with mixing of order of coordinates, as in geography latitude usually
//! goes first, but in geometry nobody puts *Y* before *X*, and ending with euclidean operations being applied to
//! angular coordinates.
//!
//! Because of that, `galileo-types` crate makes strong distinction between geographic and cartesian coordinates. Basic
//! trait for coordinates in any space is a point:
//! * [`GeoPoint`](geo::GeoPoint) is defined in [`geo`] module, and represents a point in geographic coordinate system
//! * [`CartesianPoint2d`](cartesian::CartesianPoint2d) is defined in [`cartesian`] module, and represents a point in cartesian coordinate system
//!   on a flat surface of the Earth (or another stellar body)
//!
//! Geometry traits are generic over point type they are constructed with.
//!
//! Unfortunately, most of existing systems do not have this distinction and so a same point type might require
//! implementation of both these traits. This creates a problem though for sometime it's difficult to know which trait's
//! methods are to be used in a given moment. To help elevate this problem, [`Disambig`] struct can be used.
//!
//! # Z, H, M, T coordinates
//!
//! GIS systems often work with 3rd and even 4th coordinates, but the meaning of those coordinates can differ between
//! coordinate systems:
//! * `Z` is usually an *up* coordinate in projected cartesian coordinate system with same units as *X* and *Y*
//! * `H` means height above surface or above datum
//! * `M` is an arbitrary *measure* coordinate
//! * `T` is a time coordinate
//!
//! Not distinguishing between those usages also brings confusion. For example, what would *distance* between two
//! points defined in *XYH* space mean? It might be euclidean distance on the flat surface, or 3d distance in
//! projection units, or 3d distance in *H* units (e.g. meters or feet).
//!
//! Because of this reason, points in every of those spaces are represented by different traits and provide
//! different set of methods with their own meaning.
//!
//! At this point, one such trait is defined:
//! * [`CartesianPoint3d`](cartesian::CartesianPoint3d) - a point in *XYZ* coordinate system, where *Z* is defined in projection units.
//!
//! # Converting between coordinate systems
//!
//! Converting between different types of coordinates is done using [`Projections`](geo::Projection).
//!
//! # Geometry types
//!
//! A subset of OGC geometry types are supported at the moment:
//! * [`GeoPoint`](geo::GeoPoint), [`CartesianPoint2d`](cartesian::CartesianPoint2d), [`CartesianPoint3d`](cartesian::CartesianPoint2d)
//!   (correspond to OGC *Point* geometry)
//! * [`MultiPoint`]
//! * [`Contour`] (corresponds to OGC *LineString* geometry with slight difference, check the trait's documentation)
//! * [`MultiContour`] (corresponds to OGC *MultiLineString* geometry)
//! * [`Polygon`]
//! * [`MultiPolygon`]
//!
//! # Implementing `Geometry` trait
//!
//! The most generic trait is [`Geometry`], which provides operations that can be done on any type of geometry.
//! Implementing this trait manually might be tedious for every geometry type, although for most use cases the
//! default implementation provided by this crate would be sufficient. Unfortunately, Rust type system doesn't allow
//! to provide blanket implementations of a trait for a set of other traits (because of possible conflicting implementations),
//! and at the same time let the user override default implementation with more specific one (because of
//! trait specialization problem).
//!
//! There is a way around those limitations though. You can use [`GeometryType`](geometry_type::GeometryType) trait to make your type, implementing
//! any of the specific geometry traits, also implement [`Geometry`] trait automatically.
//!
//! # Implementation for foreign types
//!
//! `galileo-types` provides geometry traits implementation for these crates:
//! * `geo-types` - enabled by `geo-types` feature
//! * `geojson` - enabled by `geojson` feature

pub mod cartesian;
pub mod contour;
mod disambig;
pub mod error;
pub mod geo;
pub mod geometry;
pub mod geometry_type;
pub mod impls;
mod multi_contour;
mod multi_point;
mod multi_polygon;
mod polygon;
mod segment;

#[cfg(feature = "geo-types")]
mod geo_types;

#[cfg(feature = "geojson")]
mod geojson;

pub use contour::{ClosedContour, Contour};
pub use disambig::{Disambig, Disambiguate};
pub use geometry::{CartesianGeometry2d, Geometry};
pub use multi_contour::MultiContour;
pub use multi_point::MultiPoint;
pub use multi_polygon::MultiPolygon;
pub use polygon::Polygon;
pub use segment::Segment;
