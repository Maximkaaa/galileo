//! Symbols are used to render [`Features`](super::Feature) in a [`FeatureLayer`](super::FeatureLayer).
//! [`Symbol`] trait is designed to be easy to implement, so an application may provide rendering logic for the
//! features it uses. But a few simple implementations are provided for convenience.

use num_traits::AsPrimitive;

mod arbitrary;
mod contour;
mod point;
mod polygon;

pub use arbitrary::ArbitraryGeometrySymbol;
pub use contour::SimpleContourSymbol;
pub use point::{CirclePointSymbol, ImagePointSymbol};
pub use polygon::SimplePolygonSymbol;

use crate::render::render_bundle::RenderPrimitive;
use galileo_types::cartesian::CartesianPoint3d;
use galileo_types::geometry::Geom;
use galileo_types::impls::{Contour, Polygon};

/// Symbol is used to draw a feature `F` to the map.
pub trait Symbol<F> {
    /// Converts the given `feature` with its `geometry` into set of primitives that should be rendered to the map.
    ///
    /// If a feature should not be rendered, an empty vector can be returned.
    ///
    /// There is no limit for number of primitives a single feature can be converted to. For example, a polygon can
    /// be rendered as a filled polygon (1) with an outline (2) and a label in the center (3).
    ///
    /// The `min_resolution` argument specifies the minimum map resolution that the returned primitives will be
    /// rendered with. This can be use to choose tolerances or pick entirely different rendering strategy. For example,
    /// a building may be rendered as a polygon at high resolution or as a point at low resolutions.
    fn render<'a, N, P>(
        &self,
        feature: &F,
        geometry: &'a Geom<P>,
        min_resolution: f64,
    ) -> Vec<RenderPrimitive<'a, N, P, Contour<P>, Polygon<P>>>
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N> + Clone;
}
