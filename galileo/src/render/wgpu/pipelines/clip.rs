use wgpu::{
    BindGroupLayout, CompareFunction, DepthStencilState, Device, RenderPass, RenderPipeline,
    RenderPipelineDescriptor, StencilFaceState, StencilOperation, StencilState, TextureFormat,
};

use crate::render::render_bundle::world_set::PolyVertex;
use crate::render::wgpu::pipelines::{default_pipeline_descriptor, default_targets};
use crate::render::wgpu::{DisplayInstance, WgpuVertexBuffers, DEPTH_FORMAT};
use crate::render::RenderOptions;

pub struct ClipPipeline {
    wgpu_pipeline: RenderPipeline,
    wgpu_pipeline_antialias: RenderPipeline,
}

impl ClipPipeline {
    const UNCLIP_REFERENCE: u32 = 0;
    const CLIP_STENCIL_VALUE: u32 = 1;

    pub fn create(
        device: &Device,
        format: TextureFormat,
        map_view_layout: &BindGroupLayout,
    ) -> Self {
        let buffers = [PolyVertex::wgpu_desc(), DisplayInstance::wgpu_desc()];
        let shader = device.create_shader_module(wgpu::include_wgsl!("./shaders/map_ref.wgsl"));

        let clip_stencil_state = StencilFaceState {
            compare: CompareFunction::Never,
            fail_op: StencilOperation::Replace,
            depth_fail_op: StencilOperation::Keep,
            pass_op: StencilOperation::Keep,
        };
        let targets = default_targets(format);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[map_view_layout],
            push_constant_ranges: &[],
        });

        let depth_stencil = Some(DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: false,
            depth_compare: CompareFunction::Always,
            stencil: StencilState {
                front: clip_stencil_state,
                back: clip_stencil_state,
                read_mask: 0xff,
                write_mask: 0xff,
            },
            bias: Default::default(),
        });

        let wgpu_pipeline_antialias = device.create_render_pipeline(&RenderPipelineDescriptor {
            depth_stencil: depth_stencil.clone(),
            ..default_pipeline_descriptor(&layout, &shader, &targets, &buffers, true)
        });
        let wgpu_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            depth_stencil,
            ..default_pipeline_descriptor(&layout, &shader, &targets, &buffers, false)
        });

        Self {
            wgpu_pipeline,
            wgpu_pipeline_antialias,
        }
    }

    pub fn clip<'a>(
        &'a self,
        buffers: &'a WgpuVertexBuffers,
        render_pass: &mut RenderPass<'a>,
        render_options: RenderOptions,
        bundle_index: u32,
    ) {
        self.render(
            buffers,
            render_pass,
            Self::CLIP_STENCIL_VALUE,
            render_options,
            bundle_index,
        );
    }

    pub fn unclip<'a>(
        &'a self,
        buffers: &'a WgpuVertexBuffers,
        render_pass: &mut RenderPass<'a>,
        render_options: RenderOptions,
        bundle_index: u32,
    ) {
        self.render(
            buffers,
            render_pass,
            Self::UNCLIP_REFERENCE,
            render_options,
            bundle_index,
        );
    }

    fn render<'a>(
        &'a self,
        buffers: &'a WgpuVertexBuffers,
        render_pass: &mut RenderPass<'a>,
        stencil_reference: u32,
        render_options: RenderOptions,
        bundle_index: u32,
    ) {
        if render_options.antialias {
            render_pass.set_pipeline(&self.wgpu_pipeline_antialias);
        } else {
            render_pass.set_pipeline(&self.wgpu_pipeline);
        }

        render_pass.set_stencil_reference(stencil_reference);
        render_pass.set_vertex_buffer(0, buffers.vertex.slice(..));
        render_pass.set_index_buffer(buffers.index.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..buffers.index_count, 0, bundle_index..(bundle_index + 1));
    }
}
