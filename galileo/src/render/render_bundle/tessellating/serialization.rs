use crate::decoded_image::DecodedImage;
use crate::render::render_bundle::tessellating::{
    ImageInfo, ImageStoreInfo, PolyVertex, PrimitiveInfo, ScreenRefVertex, TessellatingRenderBundle,
};
use lyon::lyon_tessellation::VertexBuffers;
use serde::{Deserialize, Serialize};
use std::mem::size_of;
use std::sync::Arc;

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct TessellatingRenderBundleBytes {
    pub poly_tessellation: PolyVertexBuffersBytes,
    pub points: Vec<u32>,
    pub screen_ref: ScreenRefVertexBuffersBytes,
    pub images: Vec<Option<ImageBytes>>,
    pub primitives: Vec<PrimitiveInfo>,
    pub image_store: Vec<Option<(u32, u32, Vec<u8>)>>,
    pub vacant_image_ids: Vec<usize>,
    pub vacant_image_store_ids: Vec<usize>,
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

impl TessellatingRenderBundle {
    pub(crate) fn into_bytes(self) -> TessellatingRenderBundleBytes {
        TessellatingRenderBundleBytes {
            poly_tessellation: self.poly_tessellation.into(),
            points: bytemuck::cast_vec(self.points),
            screen_ref: self.screen_ref.into(),
            images: self
                .images
                .into_iter()
                .map(|image_info| match image_info {
                    ImageInfo::Vacant => None,
                    ImageInfo::Image((image_index, vertices)) => Some(ImageBytes {
                        image_index,
                        vertices: bytemuck::cast_vec(vertices.to_vec()),
                    }),
                })
                .collect(),
            primitives: self.primitives,
            image_store: self
                .image_store
                .into_iter()
                .map(|image_info| match image_info {
                    ImageStoreInfo::Vacant => None,
                    ImageStoreInfo::Image(image) => {
                        Some((image.dimensions.0, image.dimensions.1, image.bytes.clone()))
                    }
                })
                .collect(),
            vacant_image_ids: self.vacant_image_ids,
            vacant_image_store_ids: self.vacant_image_store_ids,
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
                .map(|item| match item {
                    Some(ImageBytes {
                        image_index,
                        vertices,
                    }) => {
                        let vertices = bytemuck::cast_vec(vertices)
                            .try_into()
                            .expect("invalid vector length");

                        ImageInfo::Image((image_index, vertices))
                    }
                    None => ImageInfo::Vacant,
                })
                .collect(),
            primitives: bundle.primitives,
            image_store: bundle
                .image_store
                .into_iter()
                .map(|stored| match stored {
                    Some((width, height, bytes)) => ImageStoreInfo::Image(Arc::new(DecodedImage {
                        bytes,
                        dimensions: (width, height),
                    })),
                    None => ImageStoreInfo::Vacant,
                })
                .collect(),
            vacant_image_ids: bundle.vacant_image_ids,
            vacant_image_store_ids: bundle.vacant_image_store_ids,
            clip_area: bundle.clip_area.map(|v| v.into_typed_unchecked()),
            buffer_size: bundle.bundle_size,
            vacant_ids: vec![],
        }
    }
}
