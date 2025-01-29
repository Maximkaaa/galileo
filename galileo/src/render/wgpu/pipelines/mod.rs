use std::mem::size_of;

use wgpu::{
    BindGroup, Buffer, CompareFunction, DepthStencilState, Device, PipelineLayout, RenderPass,
    RenderPipelineDescriptor, ShaderModule, StencilFaceState, StencilOperation, StencilState,
    TextureFormat, VertexBufferLayout,
};

use crate::render::wgpu::pipelines::clip::ClipPipeline;
use crate::render::wgpu::pipelines::dot::DotPipeline;
use crate::render::wgpu::pipelines::image::ImagePipeline;
use crate::render::wgpu::pipelines::map_ref::MapRefPipeline;
use crate::render::wgpu::pipelines::screen_ref::ScreenRefPipeline;
use crate::render::wgpu::{ViewUniform, WgpuPackedBundle, DEPTH_FORMAT};
use crate::render::RenderOptions;

mod clip;
mod dot;
pub mod image;
mod map_ref;
mod screen_ref;

pub struct Pipelines {
    map_view_binding: BindGroup,
    map_view_buffer: Buffer,

    image: ImagePipeline,
    screen_ref: ScreenRefPipeline,
    map_ref: MapRefPipeline,
    clip: ClipPipeline,
    dot: DotPipeline,
}

impl Pipelines {
    pub fn create(device: &Device, format: TextureFormat) -> Self {
        let map_view_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Map view buffer"),
            size: size_of::<ViewUniform>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let map_view_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: None,
            });

        let map_view_binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &map_view_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: map_view_buffer.as_entire_binding(),
            }],
            label: Some("view_bind_group"),
        });

        Self {
            map_view_binding,
            map_view_buffer,
            image: ImagePipeline::create(device, format, &map_view_bind_group_layout),
            map_ref: MapRefPipeline::create(device, format, &map_view_bind_group_layout),
            screen_ref: ScreenRefPipeline::create(device, format, &map_view_bind_group_layout),
            clip: ClipPipeline::create(device, format, &map_view_bind_group_layout),
            dot: DotPipeline::create(device, format, &map_view_bind_group_layout),
        }
    }

    pub fn render<'a>(
        &'a self,
        render_pass: &mut RenderPass<'a>,
        bundle: &'a WgpuPackedBundle,
        render_options: RenderOptions,
        bundle_index: u32,
    ) {
        self.set_bindings(render_pass);

        if let Some(clip) = &bundle.clip_area_buffers {
            self.clip.clip(clip, render_pass, render_options);
        }

        for image in &bundle.image_buffers {
            self.image
                .render(image, render_pass, render_options, bundle_index);
        }

        if bundle.map_ref_buffers.index_count > 0 {
            self.map_ref.render(
                &bundle.map_ref_buffers,
                render_pass,
                render_options,
                bundle_index,
            );
        }

        if let Some(clip) = &bundle.clip_area_buffers {
            self.clip.unclip(clip, render_pass, render_options);
        }

        if let Some(screen_ref_buffers) = &bundle.screen_ref_buffers {
            self.screen_ref.render(
                screen_ref_buffers,
                render_pass,
                render_options,
                bundle_index,
            );
        }

        if let Some(dot_buffers) = &bundle.dot_buffers {
            self.dot
                .render(dot_buffers, render_pass, render_options, bundle_index);
        }
    }

    pub fn map_view_buffer(&self) -> &Buffer {
        &self.map_view_buffer
    }

    pub fn image_pipeline(&self) -> &ImagePipeline {
        &self.image
    }

    fn set_bindings<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        render_pass.set_bind_group(0, &self.map_view_binding, &[]);
    }
}

fn default_targets(format: TextureFormat) -> [Option<wgpu::ColorTargetState>; 1] {
    [Some(wgpu::ColorTargetState {
        format,
        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
        write_mask: wgpu::ColorWrites::ALL,
    })]
}

fn default_pipeline_descriptor<'a>(
    pipeline_layout: &'a PipelineLayout,
    shader: &'a ShaderModule,
    targets: &'a [Option<wgpu::ColorTargetState>],
    buffers: &'a [VertexBufferLayout<'a>],
    antialias: bool,
) -> RenderPipelineDescriptor<'a> {
    let stencil_state = StencilFaceState {
        compare: CompareFunction::Equal,
        fail_op: StencilOperation::Keep,
        depth_fail_op: StencilOperation::Keep,
        pass_op: StencilOperation::Keep,
    };

    RenderPipelineDescriptor {
        label: None,
        layout: Some(pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers,
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            targets,
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
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
        multisample: wgpu::MultisampleState {
            count: if antialias { 4 } else { 1 },
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: Default::default(),
    }
}
