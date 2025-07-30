use std::sync::Arc;

use wgpu::util::DeviceExt;
use wgpu::{BindGroup, BindGroupLayout, Device, RenderPass, RenderPipeline, TextureFormat};

use super::image::WgpuImage;
use crate::render::render_bundle::screen_set::ScreenSetImageVertex;
use crate::render::wgpu::pipelines::{default_pipeline_descriptor, default_targets};
use crate::render::wgpu::DisplayInstance;

const INDICES: &[u16] = &[1, 0, 2, 1, 2, 3];

pub struct ScreenSetImagePipeline {
    wgpu_pipeline: RenderPipeline,
    index_buffer: wgpu::Buffer,
}

impl ScreenSetImageVertex {
    fn wgpu_desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<ScreenSetImageVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

impl ScreenSetImagePipeline {
    pub fn create(
        device: &Device,
        format: TextureFormat,
        map_view_layout: &BindGroupLayout,
        texture_bind_group_layout: &BindGroupLayout,
    ) -> Self {
        let buffers = [
            ScreenSetImageVertex::wgpu_desc(),
            DisplayInstance::wgpu_desc(),
        ];
        let shader =
            device.create_shader_module(wgpu::include_wgsl!("./shaders/screen_set_image.wgsl"));

        let targets = default_targets(format);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[map_view_layout, texture_bind_group_layout],
            push_constant_ranges: &[],
        });
        let mut desc = default_pipeline_descriptor(&layout, &shader, &targets, &buffers, false);

        desc.multisample.count = 4;
        let wgpu_pipeline = device.create_render_pipeline(&desc);

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Image index buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            wgpu_pipeline,
            index_buffer,
        }
    }

    pub fn create_image(
        &self,
        device: &Device,
        texture: Arc<BindGroup>,
        vertices: &[ScreenSetImageVertex; 4],
    ) -> WgpuImage {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Image vertex buffer"),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(vertices),
        });

        WgpuImage {
            texture_bind_group: texture,
            vertex_buffer,
        }
    }

    pub fn render<'a>(
        &'a self,
        image: &'a WgpuImage,
        render_pass: &mut RenderPass<'a>,
        bundle_index: u32,
    ) {
        let bind_group: &BindGroup = &image.texture_bind_group;
        render_pass.set_bind_group(1, bind_group, &[]);

        render_pass.set_pipeline(&self.wgpu_pipeline);
        render_pass.set_vertex_buffer(0, image.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..INDICES.len() as u32, 0, bundle_index..(bundle_index + 1));
    }
}
