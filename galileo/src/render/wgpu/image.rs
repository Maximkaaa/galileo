use std::any::Any;
use wgpu::util::DeviceExt;
use wgpu::{BindGroupLayout, Device, Queue};

use crate::bounding_box::BoundingBox;
use crate::primitives::{DecodedImage, Image};
use crate::render::wgpu::WgpuRenderer;

const INDICES: &[u16] = &[1, 0, 2, 1, 2, 3];

pub struct ImagePainter {
    pipeline: wgpu::RenderPipeline,

    index_buffer: wgpu::Buffer,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
}

pub struct WgpuImage {
    texture: wgpu::Texture,
    texture_bind_group: wgpu::BindGroup,
    vertices: [ImageVertex; 4],
    vertex_buffer: wgpu::Buffer,
    bbox: BoundingBox,
}

impl Image for WgpuImage {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ImagePainter {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        map_view_bind_group_layout: &BindGroupLayout,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("../wgpu_shaders/image.wgsl"));

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

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Image Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout, map_view_bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Image Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[ImageVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Image index buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            pipeline,
            index_buffer,
            texture_bind_group_layout,
        }
    }

    pub fn create_image(
        &self,
        device: &Device,
        queue: &Queue,
        image: &DecodedImage,
        bbox: BoundingBox,
    ) -> WgpuImage {
        let texture_size = wgpu::Extent3d {
            width: image.dimensions.0,
            height: image.dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                label: None,
                view_formats: &[],
            },
            &image.bytes,
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
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

        let vertices = [
            ImageVertex {
                position: [bbox.x_min() as f32, bbox.y_min() as f32],
                opacity: 1.0,
                tex_coords: [0.0, 1.0],
            },
            ImageVertex {
                position: [bbox.x_min() as f32, bbox.y_max() as f32],
                opacity: 1.0,
                tex_coords: [0.0, 0.0],
            },
            ImageVertex {
                position: [bbox.x_max() as f32, bbox.y_min() as f32],
                opacity: 1.0,
                tex_coords: [1.0, 1.0],
            },
            ImageVertex {
                position: [bbox.x_max() as f32, bbox.y_max() as f32],
                opacity: 1.0,
                tex_coords: [1.0, 0.0],
            },
        ];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Image vertex buffer"),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&vertices),
        });

        WgpuImage {
            texture,
            texture_bind_group,
            vertices,
            vertex_buffer,
            bbox,
        }
    }

    pub fn draw_image<'painter, 'pass, 'image>(
        &'painter self,
        render_pass: &mut wgpu::RenderPass<'pass>,
        image: &'image WgpuImage,
        renderer: &'pass WgpuRenderer,
    ) where
        'painter: 'pass,
        'image: 'pass,
    {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &image.texture_bind_group, &[]);
        render_pass.set_bind_group(1, &renderer.map_view_bind_group, &[]);
        render_pass.set_vertex_buffer(0, image.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..INDICES.len() as u32, 0, 0..1);
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ImageVertex {
    position: [f32; 2],
    opacity: f32,
    tex_coords: [f32; 2],
}

impl ImageVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ImageVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 2]>() + std::mem::size_of::<f32>())
                        as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}
