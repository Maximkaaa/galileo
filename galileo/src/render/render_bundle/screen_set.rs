use std::sync::Arc;

use galileo_types::cartesian::{CartesianPoint3d, Point2, Rect, Vector2};
use lyon::tessellation::VertexBuffers;
use num_traits::AsPrimitive;
use serde::{Deserialize, Serialize};
use web_time::{Duration, Instant};

use crate::decoded_image::DecodedImage;
use crate::render::point_paint::MarkerStyle;
use crate::render::text::{TextService, TextShaping, TextStyle};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ScreenRenderSet {
    pub(crate) animation_duration: Duration,
    pub(crate) anchor_point: [f32; 3],
    pub(crate) bbox: Rect<f32>,
    pub(crate) hide_on_overlay: bool,
    pub(crate) data: ScreenSetData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum ScreenSetData {
    Vertices(VertexBuffers<ScreenSetVertex, u32>),
    Image {
        vertices: [ScreenSetImageVertex; 4],
        bitmap: Arc<DecodedImage>,
    },
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

#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize)]
#[repr(C)]
pub(crate) struct ScreenSetVertex {
    pub(crate) position: [f32; 2],
    pub(crate) color: [u8; 4],
}

#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize)]
#[repr(C)]
pub(crate) struct ScreenSetImageVertex {
    pub(crate) position: [f32; 2],
    pub(crate) tex_coords: [f32; 2],
}

impl ScreenRenderSet {
    pub(crate) fn new_from_label<N, P>(
        position: &P,
        text: &str,
        style: &TextStyle,
        offset: Vector2<f32>,
        dpi_scale_factor: f32,
    ) -> Option<Self>
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
    {
        match TextService::shape(text, style, offset, dpi_scale_factor) {
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
                    animation_duration: Duration::from_millis(300),
                    anchor_point: [position.x().as_(), position.y().as_(), position.z().as_()],
                    bbox,
                    hide_on_overlay: true,
                    data: ScreenSetData::Vertices(VertexBuffers { vertices, indices }),
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

    pub(crate) fn new_from_marker<N, P>(position: &P, style: &MarkerStyle) -> Option<Self>
    where
        N: AsPrimitive<f32>,
        P: CartesianPoint3d<Num = N>,
    {
        match style {
            MarkerStyle::Image {
                image,
                anchor,
                size,
            } => {
                let size = size.unwrap_or(image.size()).cast::<f32>();
                let anchor_px = *anchor * size;
                let bbox = Rect::new(
                    -anchor_px.dx(),
                    anchor_px.dy(),
                    size.width() - anchor_px.dx(),
                    anchor_px.dy() - size.height(),
                );

                let vertices = [
                    ScreenSetImageVertex {
                        position: [bbox.x_min(), bbox.y_min()],
                        tex_coords: [0.0, 1.0],
                    },
                    ScreenSetImageVertex {
                        position: [bbox.x_min(), bbox.y_max()],
                        tex_coords: [0.0, 0.0],
                    },
                    ScreenSetImageVertex {
                        position: [bbox.x_max(), bbox.y_min()],
                        tex_coords: [1.0, 1.0],
                    },
                    ScreenSetImageVertex {
                        position: [bbox.x_max(), bbox.y_max()],
                        tex_coords: [1.0, 0.0],
                    },
                ];

                Some(Self {
                    animation_duration: Duration::from_millis(0),
                    anchor_point: [position.x().as_(), position.y().as_(), position.z().as_()],
                    bbox,
                    hide_on_overlay: false,
                    data: ScreenSetData::Image {
                        vertices,
                        bitmap: image.clone(),
                    },
                })
            }
        }
    }
}
