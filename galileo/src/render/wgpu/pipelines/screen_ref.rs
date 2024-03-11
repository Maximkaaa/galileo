use crate::render::render_bundle::tessellating::ScreenRefVertex;
use crate::render::wgpu::pipelines::{default_pipeline_descriptor, default_targets};
use crate::render::wgpu::{ScreenRefBuffers, DEPTH_FORMAT};
use crate::render::RenderOptions;
use std::mem::size_of;
use wgpu::{
    BindGroupLayout, CompareFunction, DepthStencilState, Device, RenderPass, RenderPipeline,
    RenderPipelineDescriptor, StencilFaceState, StencilOperation, StencilState, TextureFormat,
};

pub struct ScreenRefPipeline {
    wgpu_pipeline: RenderPipeline,
    pub wgpu_pipeline_antialias: RenderPipeline,
}

impl ScreenRefPipeline {
    pub fn create(
        device: &Device,
        format: TextureFormat,
        map_view_layout: &BindGroupLayout,
    ) -> Self {
        let buffers = [ScreenRefVertex::wgpu_desc()];
        let shader = device.create_shader_module(wgpu::include_wgsl!("./shaders/screen_ref.wgsl"));

        let targets = default_targets(format);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[map_view_layout],
            push_constant_ranges: &[],
        });
        let stencil_state = StencilFaceState {
            compare: CompareFunction::Always,
            fail_op: StencilOperation::Keep,
            depth_fail_op: StencilOperation::Keep,
            pass_op: StencilOperation::Keep,
        };
        let mut desc = RenderPipelineDescriptor {
            depth_stencil: Some(DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Always,
                stencil: StencilState {
                    front: stencil_state,
                    back: stencil_state,
                    read_mask: 0xff,
                    write_mask: 0xff,
                },
                bias: Default::default(),
            }),
            ..default_pipeline_descriptor(&layout, &shader, &targets, &buffers, false)
        };

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
        buffers: &'a ScreenRefBuffers,
        render_pass: &mut RenderPass<'a>,
        render_options: RenderOptions,
    ) {
        if render_options.antialias {
            render_pass.set_pipeline(&self.wgpu_pipeline_antialias);
        } else {
            render_pass.set_pipeline(&self.wgpu_pipeline);
        }
        render_pass.set_vertex_buffer(0, buffers.vertex.slice(..));
        render_pass.set_index_buffer(buffers.index.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..buffers.index_count, 0, 0..1);
    }
}

impl ScreenRefVertex {
    fn wgpu_desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<ScreenRefVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: (size_of::<[f32; 3]>() + size_of::<[f32; 2]>()) as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Uint8x4,
                },
            ],
        }
    }
}
