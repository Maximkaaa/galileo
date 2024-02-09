use num_traits::AsPrimitive;

pub mod arbitrary;
pub mod contour;
pub mod point;
pub mod polygon;

use crate::render::render_bundle::RenderPrimitive;
pub use contour::SimpleContourSymbol;
use galileo_types::cartesian::impls::contour::Contour;
use galileo_types::cartesian::impls::polygon::Polygon;
use galileo_types::cartesian::traits::cartesian_point::CartesianPoint3d;
use galileo_types::geometry::Geom;
pub use point::CirclePointSymbol;
pub use polygon::SimplePolygonSymbol;

pub trait Symbol<F> {
    fn render<'a, N, P>(
        &self,
        feature: &F,
        geometry: &'a Geom<P>,
        min_resolution: f64,
    ) -> Vec<RenderPrimitive<'a, N, P, Contour<P>, Polygon<P>>>
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N> + Clone;

    fn use_antialiasing(&self) -> bool {
        true
    }
}
