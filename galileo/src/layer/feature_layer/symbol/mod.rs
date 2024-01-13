use crate::render::{PrimitiveId, RenderBundle, UnpackedBundle};

pub mod contour;
pub mod point;
pub mod polygon;

pub use contour::SimpleContourSymbol;
pub use point::CirclePointSymbol;
pub use polygon::SimplePolygonSymbol;

pub trait Symbol<F, G> {
    fn render(
        &self,
        feature: &F,
        geometry: &G,
        bundle: &mut Box<dyn RenderBundle>,
    ) -> Vec<PrimitiveId>;
    fn update(
        &self,
        _feature: &F,
        _renders_ids: &[PrimitiveId],
        _bundle: &mut Box<dyn UnpackedBundle>,
    ) {
        // provide implementation to make features editable
    }
}
