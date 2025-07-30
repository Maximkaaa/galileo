use std::mem::size_of;

use wgpu::{BindGroupLayout, Device, RenderPass, RenderPipeline, TextureFormat};

use crate::render::render_bundle::screen_set::ScreenSetVertex;
use crate::render::wgpu::pipelines::{default_pipeline_descriptor, default_targets};
use crate::render::wgpu::{DisplayInstance, WgpuVertexBuffers};

pub struct ScreenSetPipeline {
    wgpu_pipeline: RenderPipeline,
}

impl ScreenSetVertex {
    fn wgpu_desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<ScreenSetVertex>() as wgpu::BufferAddress,
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
                    format: wgpu::VertexFormat::Uint8x4,
                },
            ],
        }
    }
}

impl ScreenSetPipeline {
    pub fn create(
        device: &Device,
        format: TextureFormat,
        map_view_layout: &BindGroupLayout,
    ) -> Self {
        let buffers = [ScreenSetVertex::wgpu_desc(), DisplayInstance::wgpu_desc()];
        let shader = device.create_shader_module(wgpu::include_wgsl!("./shaders/screen_set.wgsl"));

        let targets = default_targets(format);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[map_view_layout],
            push_constant_ranges: &[],
        });
        let mut desc = default_pipeline_descriptor(&layout, &shader, &targets, &buffers, false);

        desc.multisample.count = 4;
        let wgpu_pipeline = device.create_render_pipeline(&desc);

        Self { wgpu_pipeline }
    }

    pub fn render<'a>(
        &'a self,
        buffers: &'a WgpuVertexBuffers,
        render_pass: &mut RenderPass<'a>,
        bundle_index: u32,
    ) {
        render_pass.set_pipeline(&self.wgpu_pipeline);
        render_pass.set_vertex_buffer(0, buffers.vertex.slice(..));
        render_pass.set_index_buffer(buffers.index.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..buffers.index_count, 0, bundle_index..(bundle_index + 1));
    }
}
