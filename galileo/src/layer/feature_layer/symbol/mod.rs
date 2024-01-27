use crate::render::PrimitiveId;
use num_traits::AsPrimitive;

pub mod contour;
pub mod point;
pub mod polygon;

use crate::render::render_bundle::RenderBundle;
pub use contour::SimpleContourSymbol;
use galileo_types::cartesian::traits::cartesian_point::CartesianPoint3d;
use galileo_types::geometry::Geom;
pub use point::CirclePointSymbol;
pub use polygon::SimplePolygonSymbol;

pub trait Symbol<F> {
    fn render<N: AsPrimitive<f32>, P: CartesianPoint3d<Num = N>>(
        &self,
        feature: &F,
        geometry: &Geom<P>,
        bundle: &mut RenderBundle,
        min_resolution: f64,
    ) -> Vec<PrimitiveId>;

    fn update(&self, _feature: &F, _renders_ids: &[PrimitiveId], _bundle: &mut RenderBundle) {
        // provide implementation to make features editable
    }

    fn use_antialiasing(&self) -> bool {
        true
    }
}
