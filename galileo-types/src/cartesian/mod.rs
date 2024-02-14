//! Types and functions on geometries in cartesian coordinates.

mod impls;
mod orient;
mod rect;
mod size;
mod traits;

pub use impls::{Point2, Point2d, Point3, Point3d};
pub use orient::Orientation;
pub use rect::Rect;
pub use size::Size;
pub use traits::*;
