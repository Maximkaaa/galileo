use crate::render::render_bundle::tessellating::PolyVertex;
use crate::render::wgpu::pipelines::default_targets;
use crate::render::wgpu::{pipelines, DisplayInstance, WgpuPolygonBuffers};
use crate::render::RenderOptions;
use wgpu::{BindGroupLayout, Device, RenderPass, RenderPipeline, TextureFormat};

pub struct MapRefPipeline {
    wgpu_pipeline: RenderPipeline,
    pub wgpu_pipeline_antialias: RenderPipeline,
}

impl MapRefPipeline {
    pub fn create(
        device: &Device,
        format: TextureFormat,
        map_view_layout: &BindGroupLayout,
    ) -> Self {
        let buffers = [PolyVertex::wgpu_desc(), DisplayInstance::wgpu_desc()];
        let shader = device.create_shader_module(wgpu::include_wgsl!("./shaders/map_ref.wgsl"));

        let targets = default_targets(format);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[map_view_layout],
            push_constant_ranges: &[],
        });
        let mut desc =
            pipelines::default_pipeline_descriptor(&layout, &shader, &targets, &buffers, false);
        let wgpu_pipeline = device.create_render_pipeline(&desc);

        desc.multisample.count = 4;
        let wgpu_pipeline_antialias = device.create_render_pipeline(&desc);

        Self {
            wgpu_pipeline,
            wgpu_pipeline_antialias,
        }
    }

    pub fn render<'a>(
        &'a self,
        buffers: &'a WgpuPolygonBuffers,
        render_pass: &mut RenderPass<'a>,
        render_options: RenderOptions,
        bundle_index: u32,
    ) {
        if render_options.antialias {
            render_pass.set_pipeline(&self.wgpu_pipeline_antialias);
        } else {
            render_pass.set_pipeline(&self.wgpu_pipeline);
        }
        render_pass.set_vertex_buffer(0, buffers.vertex.slice(..));
        render_pass.set_index_buffer(buffers.index.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..buffers.index_count, 0, bundle_index..(bundle_index + 1));
    }
}
