use crate::render::{PrimitiveId, RenderBundle, UnpackedBundle};

pub mod contour;
pub mod point;
pub mod polygon;

pub trait Symbol<F, G> {
    fn render(
        &self,
        feature: &F,
        geometry: &G,
        bundle: &mut Box<dyn RenderBundle>,
    ) -> Vec<PrimitiveId>;
    fn update(
        &self,
        feature: &F,
        renders_ids: &[PrimitiveId],
        bundle: &mut Box<dyn UnpackedBundle>,
    );
}
