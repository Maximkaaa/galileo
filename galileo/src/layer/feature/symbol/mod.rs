use crate::render::{RenderBundle, UnpackedBundle};

pub mod contour;
pub mod point;
pub mod polygon;

pub trait Symbol<F, G> {
    fn render(&self, feature: &F, geometry: &G, bundle: &mut Box<dyn RenderBundle>) -> Vec<usize>;
    fn update(&self, feature: &F, renders_ids: &[usize], bundle: &mut Box<dyn UnpackedBundle>);
}
