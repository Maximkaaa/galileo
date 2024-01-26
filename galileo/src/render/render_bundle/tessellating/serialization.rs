use crate::primitives::DecodedImage;
use crate::render::render_bundle::tessellating::{
    LodTessellation, PolyVertex, PrimitiveInfo, ScreenRefVertex, TessellatingRenderBundle,
};
use lyon::lyon_tessellation::VertexBuffers;
use serde::{Deserialize, Serialize};
use std::mem::size_of;

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct TessellatingRenderBundleBytes {
    pub poly_tessellation: Vec<LodTessellationBytes>,
    pub points: Vec<u32>,
    pub screen_ref: ScreenRefVertexBuffersBytes,
    pub images: Vec<ImageBytes>,
    pub primitives: Vec<PrimitiveInfo>,
    pub clip_area: Option<PolyVertexBuffersBytes>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ImageBytes {
    image_bytes: Vec<u8>,
    dimensions: (u32, u32),
    vertices: Vec<u32>,
}

const POLY_VERTEX_BLOCKS: usize = size_of::<PolyVertex>() / size_of::<u32>();

type PolyVertexShim = [u32; POLY_VERTEX_BLOCKS];

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct PolyVertexBuffersBytes {
    vertices: Vec<PolyVertexShim>,
    indices: Vec<u32>,
}

impl From<VertexBuffers<PolyVertex, u32>> for PolyVertexBuffersBytes {
    fn from(value: VertexBuffers<PolyVertex, u32>) -> Self {
        Self {
            vertices: bytemuck::cast_vec(value.vertices),
            indices: bytemuck::cast_vec(value.indices),
        }
    }
}

impl PolyVertexBuffersBytes {
    fn into_typed_unchecked(self) -> VertexBuffers<PolyVertex, u32> {
        let vertices = bytemuck::cast_vec(self.vertices);
        let indices = bytemuck::cast_vec(self.indices);

        VertexBuffers { vertices, indices }
    }
}

const SCREEN_REF_VERTEX_BLOCKS: usize = size_of::<ScreenRefVertex>() / size_of::<u32>();
type ScreenRefVertexShim = [u32; SCREEN_REF_VERTEX_BLOCKS];

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ScreenRefVertexBuffersBytes {
    vertices: Vec<ScreenRefVertexShim>,
    indices: Vec<u32>,
}

impl From<VertexBuffers<ScreenRefVertex, u32>> for ScreenRefVertexBuffersBytes {
    fn from(value: VertexBuffers<ScreenRefVertex, u32>) -> Self {
        Self {
            vertices: bytemuck::cast_vec(value.vertices),
            indices: value.indices,
        }
    }
}

impl ScreenRefVertexBuffersBytes {
    fn into_typed_unchecked(self) -> VertexBuffers<ScreenRefVertex, u32> {
        let vertices = bytemuck::cast_vec(self.vertices);
        let indices = bytemuck::cast_vec(self.indices);

        VertexBuffers { vertices, indices }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct LodTessellationBytes {
    pub min_resolution: f32,
    pub tessellation: PolyVertexBuffersBytes,
}

impl From<LodTessellation> for LodTessellationBytes {
    fn from(value: LodTessellation) -> Self {
        Self {
            min_resolution: value.min_resolution,
            tessellation: value.tessellation.into(),
        }
    }
}

impl LodTessellation {
    fn from_bytes_unchecked(lod: LodTessellationBytes) -> Self {
        Self {
            min_resolution: lod.min_resolution,
            tessellation: lod.tessellation.into_typed_unchecked(),
        }
    }
}

impl TessellatingRenderBundle {
    pub(crate) fn into_bytes(self) -> TessellatingRenderBundleBytes {
        let converted = TessellatingRenderBundleBytes {
            poly_tessellation: self
                .poly_tessellation
                .into_iter()
                .map(|v| v.into())
                .collect(),
            points: bytemuck::cast_vec(self.points),
            screen_ref: self.screen_ref.into(),
            images: self
                .images
                .into_iter()
                .map(|(image, vertices)| ImageBytes {
                    image_bytes: bytemuck::cast_vec(image.bytes),
                    dimensions: image.dimensions,
                    vertices: bytemuck::cast_vec(vertices.to_vec()),
                })
                .collect(),
            primitives: self.primitives,
            clip_area: self.clip_area.map(|v| v.into()),
        };

        converted
    }

    pub(crate) fn from_bytes_unchecked(bundle: TessellatingRenderBundleBytes) -> Self {
        Self {
            poly_tessellation: bundle
                .poly_tessellation
                .into_iter()
                .map(|v| LodTessellation::from_bytes_unchecked(v))
                .collect(),
            points: bytemuck::cast_vec(bundle.points),
            screen_ref: bundle.screen_ref.into_typed_unchecked(),
            images: bundle
                .images
                .into_iter()
                .map(|v| {
                    let decoded_image = DecodedImage {
                        bytes: v.image_bytes,
                        dimensions: v.dimensions,
                    };
                    let vertices = bytemuck::cast_vec(v.vertices)
                        .try_into()
                        .expect("invalid vector length");

                    (decoded_image, vertices)
                })
                .collect(),
            primitives: bundle.primitives,
            clip_area: bundle.clip_area.map(|v| v.into_typed_unchecked()),
        }
    }
}
