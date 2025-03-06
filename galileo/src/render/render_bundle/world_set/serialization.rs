use std::mem::size_of;
use std::sync::Arc;

use galileo_types::cartesian::Size;
use lyon::lyon_tessellation::VertexBuffers;
use serde::{Deserialize, Serialize};

use crate::decoded_image::{DecodedImage, DecodedImageType};
use crate::render::render_bundle::world_set::{
    ImageInfo, PolyVertex, ScreenRefVertex, WorldRenderSet,
};

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct TessellatingRenderBundleBytes {
    pub poly_tessellation: PolyVertexBuffersBytes,
    pub points: Vec<u32>,
    pub screen_ref: ScreenRefVertexBuffersBytes,
    pub images: Vec<ImageBytes>,
    pub image_store: Vec<(u32, u32, Vec<u8>)>,
    pub clip_area: Option<PolyVertexBuffersBytes>,
    pub bundle_size: usize,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ImageBytes {
    image_index: usize,
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

impl WorldRenderSet {
    pub(crate) fn into_bytes(self) -> TessellatingRenderBundleBytes {
        TessellatingRenderBundleBytes {
            poly_tessellation: self.poly_tessellation.into(),
            points: bytemuck::cast_vec(self.points),
            screen_ref: self.screen_ref.into(),
            images: self
                .images
                .into_iter()
                .map(|image_info| ImageBytes {
                    image_index: image_info.store_index,
                    vertices: bytemuck::cast_vec(image_info.vertices.to_vec()),
                })
                .collect(),
            image_store: self
                .image_store
                .into_iter()
                .map(|image| match &image.0 {
                    DecodedImageType::Bitmap { bytes, dimensions } => {
                        (dimensions.width(), dimensions.height(), bytes.clone())
                    }
                    #[cfg(target_arch = "wasm32")]
                    _ => panic!("only supported for raw bitmaps"),
                })
                .collect(),
            clip_area: self.clip_area.map(|v| v.into()),
            bundle_size: self.buffer_size,
        }
    }

    pub(crate) fn from_bytes_unchecked(bundle: TessellatingRenderBundleBytes) -> Self {
        Self {
            poly_tessellation: bundle.poly_tessellation.into_typed_unchecked(),
            points: bytemuck::cast_vec(bundle.points),
            screen_ref: bundle.screen_ref.into_typed_unchecked(),
            images: bundle
                .images
                .into_iter()
                .map(|item| {
                    let vertices = bytemuck::cast_vec(item.vertices)
                        .try_into()
                        .expect("invalid vector length");

                    ImageInfo {
                        store_index: item.image_index,
                        vertices,
                    }
                })
                .collect(),
            image_store: bundle
                .image_store
                .into_iter()
                .map(|stored| {
                    Arc::new(DecodedImage(DecodedImageType::Bitmap {
                        bytes: stored.2,
                        dimensions: Size::new(stored.0, stored.1),
                    }))
                })
                .collect(),
            clip_area: bundle.clip_area.map(|v| v.into_typed_unchecked()),
            buffer_size: bundle.bundle_size,
        }
    }
}
