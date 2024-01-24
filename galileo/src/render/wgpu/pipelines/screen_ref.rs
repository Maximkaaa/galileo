use crate::render::render_bundle::tessellating::PointInstance;
use crate::render::wgpu::pipelines::default_targets;
use crate::render::wgpu::{pipelines, PointVertex, WgpuPointBuffers};
use wgpu::{BindGroupLayout, Device, RenderPass, RenderPipeline, TextureFormat};

pub struct ScreenRefPipeline {
    wgpu_pipeline: RenderPipeline,
}

impl ScreenRefPipeline {
    pub fn create(
        device: &Device,
        format: TextureFormat,
        map_view_layout: &BindGroupLayout,
    ) -> Self {
        let buffers = [PointVertex::wgpu_desc(), PointInstance::wgpu_desc()];
        let shader = device.create_shader_module(wgpu::include_wgsl!("./shaders/screen_ref.wgsl"));

        let targets = default_targets(format);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[map_view_layout],
            push_constant_ranges: &[],
        });
        let desc = pipelines::default_pipeline_descriptor(&layout, &shader, &targets, &buffers);

        let wgpu_pipeline = device.create_render_pipeline(&desc);
        Self { wgpu_pipeline }
    }

    pub fn render<'a>(&'a self, buffers: &'a WgpuPointBuffers, render_pass: &mut RenderPass<'a>) {
        render_pass.set_pipeline(&self.wgpu_pipeline);
        render_pass.set_vertex_buffer(0, buffers.vertex.slice(..));
        render_pass.set_vertex_buffer(1, buffers.instance.slice(..));
        render_pass.set_index_buffer(buffers.index.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..buffers.index_count, 0, 0..buffers.instance_count);
    }
}
