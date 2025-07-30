use std::mem::size_of;
use std::sync::Arc;

use screen_set_image::ScreenSetImagePipeline;
use screen_set_vertex::ScreenSetPipeline;
use wgpu::util::{DeviceExt, TextureDataOrder};
use wgpu::{
    BindGroup, BindGroupLayout, Buffer, CompareFunction, DepthStencilState, Device, PipelineLayout,
    Queue, RenderPass, RenderPipelineDescriptor, ShaderModule, StencilFaceState, StencilOperation,
    StencilState, TextureFormat, VertexBufferLayout,
};

use super::WgpuScreenSetData;
use crate::decoded_image::{DecodedImage, DecodedImageType};
use crate::render::wgpu::pipelines::clip::ClipPipeline;
use crate::render::wgpu::pipelines::dot::DotPipeline;
use crate::render::wgpu::pipelines::image::ImagePipeline;
use crate::render::wgpu::pipelines::map_ref::MapRefPipeline;
use crate::render::wgpu::{ViewUniform, WgpuPackedBundle, DEPTH_FORMAT};
use crate::render::RenderOptions;

mod clip;
mod dot;
pub mod image;
mod map_ref;
mod screen_set_image;
mod screen_set_vertex;

pub struct Pipelines {
    map_view_binding: BindGroup,
    map_view_buffer: Buffer,
    pub(crate) map_view_bind_group_layout: BindGroupLayout,
    texture_bind_group_layout: BindGroupLayout,

    image: ImagePipeline,
    map_ref: MapRefPipeline,
    clip: ClipPipeline,
    dot: DotPipeline,
    screen_set: ScreenSetPipeline,
    screen_set_image: ScreenSetImagePipeline,
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

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_label"),
            });

        Self {
            map_view_binding,
            map_view_buffer,
            map_view_bind_group_layout: map_view_bind_group_layout.clone(),
            texture_bind_group_layout: texture_bind_group_layout.clone(),
            image: ImagePipeline::create(
                device,
                format,
                &map_view_bind_group_layout,
                &texture_bind_group_layout,
            ),
            map_ref: MapRefPipeline::create(device, format, &map_view_bind_group_layout),
            clip: ClipPipeline::create(device, format, &map_view_bind_group_layout),
            dot: DotPipeline::create(device, format, &map_view_bind_group_layout),
            screen_set: ScreenSetPipeline::create(device, format, &map_view_bind_group_layout),
            screen_set_image: ScreenSetImagePipeline::create(
                device,
                format,
                &map_view_bind_group_layout,
                &texture_bind_group_layout,
            ),
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
            self.clip
                .clip(clip, render_pass, render_options, bundle_index);
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
            self.clip
                .unclip(clip, render_pass, render_options, bundle_index);
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

    pub fn screen_set_image_pipeline(&self) -> &ScreenSetImagePipeline {
        &self.screen_set_image
    }

    pub fn set_bindings<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        render_pass.set_bind_group(0, &self.map_view_binding, &[]);
    }

    pub fn render_screen_set<'a>(
        &'a self,
        data: &'a WgpuScreenSetData,
        render_pass: &mut RenderPass<'a>,
        bundle_index: u32,
    ) {
        self.set_bindings(render_pass);
        match data {
            WgpuScreenSetData::Vertex(wgpu_vertex_buffers) => {
                self.screen_set
                    .render(wgpu_vertex_buffers, render_pass, bundle_index)
            }
            WgpuScreenSetData::Image(wgpu_image) => {
                self.screen_set_image
                    .render(wgpu_image, render_pass, bundle_index)
            }
        }
    }

    pub fn create_image_texture(
        &self,
        device: &Device,
        queue: &Queue,
        image: &DecodedImage,
    ) -> Arc<BindGroup> {
        let texture_size = wgpu::Extent3d {
            width: image.width(),
            height: image.height(),
            depth_or_array_layers: 1,
        };

        let texture = match &image.0 {
            DecodedImageType::Bitmap { bytes, .. } => device.create_texture_with_data(
                queue,
                &wgpu::TextureDescriptor {
                    size: texture_size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: TextureFormat::Rgba8UnormSrgb,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    label: None,
                    view_formats: &[],
                },
                TextureDataOrder::default(),
                bytes,
            ),
            #[cfg(target_arch = "wasm32")]
            DecodedImageType::JsImageBitmap { js_image, .. } => {
                use wgpu::{CopyExternalImageSourceInfo, ExternalImageSource, Origin2d};

                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    size: texture_size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: TextureFormat::Rgba8UnormSrgb,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_DST
                        | wgpu::TextureUsages::RENDER_ATTACHMENT,
                    label: None,
                    view_formats: &[],
                });
                let texture_size = wgpu::Extent3d {
                    width: js_image.width(),
                    height: js_image.height(),
                    depth_or_array_layers: 1,
                };
                let image = CopyExternalImageSourceInfo {
                    source: ExternalImageSource::ImageBitmap(js_image.clone()),
                    origin: Origin2d::ZERO,
                    flip_y: false,
                };
                queue.copy_external_image_to_texture(
                    &image,
                    texture
                        .as_image_copy()
                        .to_tagged(wgpu::PredefinedColorSpace::Srgb, false),
                    texture_size,
                );

                texture
            }
        };

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        });

        Arc::new(texture_bind_group)
    }
}

pub(crate) fn default_targets(format: TextureFormat) -> [Option<wgpu::ColorTargetState>; 1] {
    [Some(wgpu::ColorTargetState {
        format,
        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
        write_mask: wgpu::ColorWrites::ALL,
    })]
}

pub(crate) fn default_pipeline_descriptor<'a>(
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
