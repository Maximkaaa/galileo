//! See documentation for [`GeometryType`] trait.
use serde::{Deserialize, Serialize};

/// This trait allows automatically implement [`Geometry`](crate::Geometry) trait for types that implement specific
/// geometry traits (e.g. [`Polygon`](crate::Polygon) etc).
pub trait GeometryType {
    /// Type of the geometry. [`Geometry`](crate::Geometry) trait is implemented for one of the following types:
    /// * [`PointGeometryType`]
    /// * [`MultiPointGeometryType`]
    /// * [`ContourGeometryType`]
    /// * [`MultiContourGeometryType`]
    /// * [`PolygonGeometryType`]
    /// * [`MultiPolygonGeometryType`]
    type Type;

    /// Coordinate space that this geometry uses. This specifies what kind of coordinates the geometry uses.
    ///
    /// The defined coordinate spaces are:
    /// * [`GeoSpace2d`]
    /// * [`CartesianSpace2d`]
    /// * [`CartesianSpace3d`]
    ///
    /// Some types are not bound by the coordinate space they can represent. In this case [`AmbiguousSpace`] space can
    /// be used. These can be converted into a specific coordinate space using [`Disambiguate`](crate::Disambiguate) trait.
    type Space;
}

/// Point geometry marker.
#[derive(Debug, Copy, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Hash, Deserialize, Serialize)]
pub struct PointGeometryType;

/// Multipoint geometry marker.
#[derive(Debug, Copy, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Hash, Deserialize, Serialize)]
pub struct MultiPointGeometryType;

/// Contour geometry marker.
#[derive(Debug, Copy, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Hash, Deserialize, Serialize)]
pub struct ContourGeometryType;

/// MultiContour geometry marker.
#[derive(Debug, Copy, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Hash, Deserialize, Serialize)]
pub struct MultiContourGeometryType;

/// Polygon geometry marker.
#[derive(Debug, Copy, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Hash, Deserialize, Serialize)]
pub struct PolygonGeometryType;

/// MultiPolygon geometry marker.
#[derive(Debug, Copy, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Hash, Deserialize, Serialize)]
pub struct MultiPolygonGeometryType;

/// Geographic coordinate space marker.
#[derive(Debug, Copy, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Hash, Deserialize, Serialize)]
pub struct GeoSpace2d;

/// 2d cartesian coordinate space marker.
#[derive(Debug, Copy, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Hash, Deserialize, Serialize)]
pub struct CartesianSpace2d;

/// 3d cartesian coordinate space marker.
#[derive(Debug, Copy, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Hash, Deserialize, Serialize)]
pub struct CartesianSpace3d;

/// See [`Disambiguate`](super::disambig::Disambiguate).
#[derive(Debug, Copy, Clone, Default, PartialEq, PartialOrd, Eq, Ord, Hash, Deserialize, Serialize)]
pub struct AmbiguousSpace;
