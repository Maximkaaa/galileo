use crate::render::render_bundle::tessellating::PointInstance;
use crate::render::wgpu::pipelines::{default_pipeline_descriptor, default_targets};
use crate::render::wgpu::{DisplayInstance, WgpuDotBuffers, DEPTH_FORMAT};
use crate::render::RenderOptions;
use wgpu::{
    BindGroupLayout, CompareFunction, DepthStencilState, Device, RenderPass, RenderPipeline,
    RenderPipelineDescriptor, StencilFaceState, StencilOperation, StencilState, TextureFormat,
    VertexStepMode,
};

pub struct DotPipeline {
    wgpu_pipeline: RenderPipeline,
    pub wgpu_pipeline_antialias: RenderPipeline,
}

impl DotPipeline {
    pub fn create(
        device: &Device,
        format: TextureFormat,
        map_view_layout: &BindGroupLayout,
    ) -> Self {
        let mut desc = PointInstance::wgpu_desc();
        desc.step_mode = VertexStepMode::Vertex;

        let buffers = [desc, DisplayInstance::wgpu_desc()];
        let shader = device.create_shader_module(wgpu::include_wgsl!("./shaders/dot.wgsl"));

        let targets = default_targets(format);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[map_view_layout],
            push_constant_ranges: &[],
        });
        let stencil_state = StencilFaceState {
            compare: CompareFunction::Equal,
            fail_op: StencilOperation::Keep,
            depth_fail_op: StencilOperation::Keep,
            pass_op: StencilOperation::Keep,
        };
        let primitive = wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::PointList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        };
        let depth_stencil = Some(DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: CompareFunction::LessEqual,
            stencil: StencilState {
                front: stencil_state,
                back: stencil_state,
                read_mask: 0xff,
                write_mask: 0xff,
            },
            bias: Default::default(),
        });

        let wgpu_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            primitive,
            depth_stencil: depth_stencil.clone(),
            ..default_pipeline_descriptor(&layout, &shader, &targets, &buffers, false)
        });
        let wgpu_pipeline_antialias = device.create_render_pipeline(&RenderPipelineDescriptor {
            primitive,
            depth_stencil,
            ..default_pipeline_descriptor(&layout, &shader, &targets, &buffers, true)
        });
        Self {
            wgpu_pipeline,
            wgpu_pipeline_antialias,
        }
    }

    pub fn render<'a>(
        &'a self,
        buffers: &'a WgpuDotBuffers,
        render_pass: &mut RenderPass<'a>,
        render_options: RenderOptions,
        bundle_index: u32,
    ) {
        if render_options.antialias {
            render_pass.set_pipeline(&self.wgpu_pipeline_antialias);
        } else {
            render_pass.set_pipeline(&self.wgpu_pipeline);
        }

        render_pass.set_vertex_buffer(0, buffers.buffer.slice(..));
        render_pass.draw(0..buffers.point_count, bundle_index..(bundle_index + 1));
    }
}
