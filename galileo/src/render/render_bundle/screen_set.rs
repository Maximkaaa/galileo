use galileo_types::cartesian::{CartesianPoint3d, Rect};
use nalgebra::{Point2, Vector2};
use num_traits::AsPrimitive;
use web_time::{Duration, Instant};

use crate::render::text::{FontService, TextShaping, TextStyle};

#[derive(Debug, Clone)]
pub(crate) struct ScreenRenderSet {
    pub(crate) initial_state: RenderSetState,
    pub(crate) animation_duration: Duration,
    pub(crate) anchor_point: [f32; 3],
    pub(crate) bbox: Rect<f32>,
    pub(crate) vertices: Vec<ScreenSetVertex>,
    pub(crate) indices: Vec<u32>,
}

#[derive(Debug, Copy, Clone)]
pub(crate) enum RenderSetState {
    Hidden,
    FadingIn { start_time: Instant },
    Displayed,
    FadingOut { start_time: Instant },
}

impl RenderSetState {
    pub(crate) fn is_displayed(&self) -> bool {
        matches!(self, Self::FadingIn { .. } | Self::Displayed)
    }
}

#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub(crate) struct ScreenSetVertex {
    pub(crate) position: [f32; 2],
    pub(crate) color: [u8; 4],
}

impl ScreenRenderSet {
    pub(crate) fn new_from_label<N, P>(
        position: &P,
        text: &str,
        style: &TextStyle,
        offset: Vector2<f32>,
    ) -> Option<Self>
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
    {
        match FontService::shape(text, style, offset) {
            Ok(TextShaping::Tessellation { glyphs, .. }) => {
                let mut vertices = vec![];
                let mut indices = vec![];

                let mut bbox: Option<Rect<f32>> = None;

                for glyph in glyphs {
                    let vertices_start = vertices.len() as u32;

                    for vertex in glyph.vertices {
                        let vertex_bbox =
                            Rect::from_point(&Point2::new(vertex.position[0], vertex.position[1]));

                        bbox = match bbox {
                            Some(bbox) => Some(bbox.merge(vertex_bbox)),
                            None => Some(vertex_bbox),
                        };

                        vertices.push(ScreenSetVertex {
                            position: vertex.position,
                            color: vertex.color.to_u8_array(),
                        });
                    }

                    for index in glyph.indices {
                        indices.push(index + vertices_start);
                    }
                }

                let Some(bbox) = bbox else {
                    // No vertices, nothing to render
                    return None;
                };

                Some(Self {
                    initial_state: RenderSetState::Hidden,
                    animation_duration: Duration::from_millis(300),
                    anchor_point: [position.x().as_(), position.y().as_(), position.z().as_()],
                    bbox,
                    vertices,
                    indices,
                })
            }
            Err(err) => {
                log::error!("Error shaping text label: {err:?}");
                None
            }
            _ => {
                log::error!("Not supported font type");
                None
            }
        }
    }
}
