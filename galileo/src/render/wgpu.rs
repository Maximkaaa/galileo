use galileo_types::cartesian::size::Size;
use lyon::tessellation::VertexBuffers;
use nalgebra::{Rotation3, Vector3};
use std::any::Any;
use std::mem::size_of;
use std::ops::Range;
use wgpu::util::DeviceExt;
use wgpu::{
    BindGroup, BindGroupLayout, Buffer, CompareFunction, DepthStencilState, Device, Extent3d,
    Queue, RenderPass, RenderPassDepthStencilAttachment, StencilFaceState, StencilOperation,
    StencilState, StoreOp, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureView, TextureViewDescriptor,
};
use winit::dpi::PhysicalSize;

use crate::layer::Layer;
use crate::map::Map;
use crate::primitives::{Color, DecodedImage};
use crate::render::render_bundle::tessellating::{
    ImageVertex, LodTessellation, PointInstance, PolyVertex, PrimitiveInfo,
    TessellatingRenderBundle,
};
use crate::render::render_bundle::RenderBundle;
use crate::render::wgpu::image::WgpuImage;
use crate::view::MapView;

use self::image::ImagePainter;

use super::{
    Canvas, ImagePaint, LinePaint, PackedBundle, Paint, PointPaint, PrimitiveId, Renderer,
    UnpackedBundle,
};

pub struct WgpuRenderer {
    surface: wgpu::Surface,
    device: Device,
    queue: Queue,
    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,

    line_pipeline: wgpu::RenderPipeline,
    image_painter: ImagePainter,
    map_view_buffer: Buffer,
    map_view_bind_group: BindGroup,
    pub map_view_bind_group_layout: BindGroupLayout,
    pub multisampling_view: TextureView,
    pub point_pipeline: wgpu::RenderPipeline,

    background: Color,
    pub stencil_view: TextureView,
    pub clip_pipeline: wgpu::RenderPipeline,
}

impl Renderer for WgpuRenderer {
    fn create_bundle(&self, lods: &Option<Vec<f32>>) -> RenderBundle {
        match lods {
            Some(v) => RenderBundle::Tessellating(TessellatingRenderBundle::with_lods(v)),
            None => RenderBundle::Tessellating(TessellatingRenderBundle::new()),
        }
    }

    fn pack_bundle(&self, bundle: RenderBundle) -> Box<dyn PackedBundle> {
        match bundle {
            RenderBundle::Tessellating(inner) => Box::new(WgpuPackedBundle::new(
                inner.clip_area,
                inner.poly_tessellation,
                inner.points,
                inner.images,
                inner.primitives,
                self,
            )),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl WgpuRenderer {
    pub async fn create(window: &winit::window::Window) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            flags: Default::default(),
            dx12_shader_compiler: Default::default(),
            gles_minor_version: Default::default(),
        });

        let surface = unsafe { instance.create_surface(window) }.unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits {
                            max_texture_dimension_2d: 4096,
                            ..wgpu::Limits::downlevel_webgl2_defaults()
                        }
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &config);

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
                label: Some("map_view_bind_group_layout"),
            });

        let line_shader =
            device.create_shader_module(wgpu::include_wgsl!("./wgpu_shaders/line.wgsl"));

        let line_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Line Render Pipeline Layout"),
                bind_group_layouts: &[&map_view_bind_group_layout],
                push_constant_ranges: &[],
            });

        let poly_stencil_state = StencilFaceState {
            compare: CompareFunction::Equal,
            fail_op: StencilOperation::Keep,
            depth_fail_op: StencilOperation::Keep,
            pass_op: StencilOperation::Keep,
        };

        let targets = [Some(wgpu::ColorTargetState {
            format: config.format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        })];

        let poly_pipeline_desc = wgpu::RenderPipelineDescriptor {
            label: Some("Line Render Pipeline"),
            layout: Some(&line_render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &line_shader,
                entry_point: "vs_main",
                buffers: &[PolyVertex::wgpu_desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &line_shader,
                entry_point: "fs_main",
                targets: &targets,
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
                format: TextureFormat::Depth24PlusStencil8,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Always,
                stencil: StencilState {
                    front: poly_stencil_state,
                    back: poly_stencil_state,
                    read_mask: 0xff,
                    write_mask: 0xff,
                },
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        };

        let line_pipeline = device.create_render_pipeline(&poly_pipeline_desc);

        let clip_stencil_state = StencilFaceState {
            compare: CompareFunction::Never,
            fail_op: StencilOperation::Replace,
            depth_fail_op: StencilOperation::Keep,
            pass_op: StencilOperation::Keep,
        };
        let clip_pipeline_desc = wgpu::RenderPipelineDescriptor {
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth24PlusStencil8,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Always,
                stencil: StencilState {
                    front: clip_stencil_state,
                    back: clip_stencil_state,
                    read_mask: 0xff,
                    write_mask: 0xff,
                },
                bias: Default::default(),
            }),
            ..poly_pipeline_desc
        };
        let clip_pipeline = device.create_render_pipeline(&clip_pipeline_desc);

        let point_shader =
            device.create_shader_module(wgpu::include_wgsl!("./wgpu_shaders/point.wgsl"));

        let point_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Point Render Pipeline Layout"),
                bind_group_layouts: &[&map_view_bind_group_layout],
                push_constant_ranges: &[],
            });

        let point_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Point Render Pipeline"),
            layout: Some(&point_render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &point_shader,
                entry_point: "vs_main",
                buffers: &[PointVertex::wgpu_desc(), PointInstance::wgpu_desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &point_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
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
                format: TextureFormat::Depth24PlusStencil8,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Always,
                stencil: StencilState {
                    front: poly_stencil_state,
                    back: poly_stencil_state,
                    read_mask: 0xff,
                    write_mask: 0xff,
                },
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let map_view_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &map_view_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: map_view_buffer.as_entire_binding(),
            }],
            label: Some("view_bind_group"),
        });

        let image_painter = ImagePainter::new(&device, config.format, &map_view_bind_group_layout);
        let multisampling_view = Self::create_multisample_texture(&device, size, config.format);
        let stencil_view = Self::create_stencil_texture(&device, size);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            line_pipeline,
            clip_pipeline,
            point_pipeline,
            map_view_buffer,
            map_view_bind_group,
            map_view_bind_group_layout,
            image_painter,
            multisampling_view,
            stencil_view,
            background: Color::rgba(255, 255, 255, 255),
        }
    }

    pub fn set_background(&mut self, color: Color) {
        self.background = color;
    }

    fn create_multisample_texture(
        device: &Device,
        size: PhysicalSize<u32>,
        format: TextureFormat,
    ) -> TextureView {
        let multisampling_texture = device.create_texture(&TextureDescriptor {
            label: Some("Multisampling texture"),
            size: Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 4,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        multisampling_texture.create_view(&TextureViewDescriptor::default())
    }

    fn create_stencil_texture(device: &Device, size: PhysicalSize<u32>) -> TextureView {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("Stencil/depth texture"),
            size: Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 4,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth24PlusStencil8,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        texture.create_view(&TextureViewDescriptor::default())
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            self.multisampling_view =
                Self::create_multisample_texture(&self.device, new_size, self.config.format);
            self.stencil_view = Self::create_stencil_texture(&self.device, new_size);
        }
    }

    pub fn render(&self, map: &Map) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&TextureViewDescriptor {
            ..Default::default()
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let background = self.background.to_f32_array();
            let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.multisampling_view,
                    resolve_target: Some(&view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: background[0] as f64,
                            g: background[1] as f64,
                            b: background[2] as f64,
                            a: background[3] as f64,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        self.render_map(map, &view);

        output.present();

        Ok(())
    }

    fn render_map(&self, map: &Map, texture_view: &TextureView) {
        let view = map.view();
        for layer in map.layers() {
            self.render_layer(&(**layer), view, texture_view);
        }
    }

    fn render_layer(&self, layer: &dyn Layer, view: &MapView, texture_view: &TextureView) {
        let mut canvas = WgpuCanvas::new(self, texture_view, view.clone());
        layer.render(view, &mut canvas);
    }

    pub fn size(&self) -> Size {
        Size::new(self.size.width as f64, self.size.height as f64)
    }

    pub fn pack_bundle(&self, bundle: Box<dyn UnpackedBundle>) -> Box<dyn PackedBundle> {
        let bundle: Box<WgpuUnpackedBundle> = bundle.into_any().downcast().unwrap();
        Box::new(bundle.pack(&self.queue))
    }
}

#[allow(dead_code)]
struct WgpuCanvas<'a> {
    renderer: &'a WgpuRenderer,
    view: &'a TextureView,
}

impl<'a> WgpuCanvas<'a> {
    fn new(renderer: &'a WgpuRenderer, view: &'a TextureView, map_view: MapView) -> Self {
        let rotation_mtx = Rotation3::new(Vector3::new(
            map_view.rotation_x(),
            0.0,
            -map_view.rotation_z(),
        ))
        .to_homogeneous();
        renderer.queue.write_buffer(
            &renderer.map_view_buffer,
            0,
            bytemuck::cast_slice(&[ViewUniform {
                view_proj: map_view.map_to_scene_mtx().unwrap(),
                view_rotation: rotation_mtx.cast::<f32>().data.0,
                inv_screen_size: [
                    1.0 / renderer.size.width as f32,
                    1.0 / renderer.size.height as f32,
                ],
                resolution: map_view.resolution() as f32,
                _padding: [0.0; 1],
            }]),
        );

        Self { renderer, view }
    }
}

impl<'a> Canvas for WgpuCanvas<'a> {
    fn size(&self) -> Size {
        self.renderer.size()
    }

    fn create_bundle(&self, lods: &Option<Vec<f32>>) -> RenderBundle {
        self.renderer.create_bundle(lods)
    }

    fn pack_bundle(&self, bundle: RenderBundle) -> Box<dyn PackedBundle> {
        match bundle {
            RenderBundle::Tessellating(inner) => Box::new(WgpuPackedBundle::new(
                inner.clip_area,
                inner.poly_tessellation,
                inner.points,
                inner.images,
                inner.primitives,
                self.renderer,
            )),
        }
    }

    fn pack_unpacked(&self, bundle: Box<dyn UnpackedBundle>) -> Box<dyn PackedBundle> {
        self.renderer.pack_bundle(bundle)
    }

    fn draw_bundles(&mut self, bundles: &[&dyn PackedBundle], resolution: f32) {
        let mut encoder =
            self.renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.renderer.multisampling_view,
                    resolve_target: Some(self.view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.renderer.stencil_view,
                    depth_ops: None,
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0),
                        store: StoreOp::Discard,
                    }),
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            for bundle in bundles {
                if let Some(cast) = bundle.as_any().downcast_ref::<WgpuPackedBundle>() {
                    if let Some(clip_area_buffers) = &cast.clip_area_buffers {
                        render_pass.set_stencil_reference(1);
                        render_pass.set_pipeline(&self.renderer.clip_pipeline);
                        render_pass.set_bind_group(0, &self.renderer.map_view_bind_group, &[]);
                        render_pass.set_vertex_buffer(0, clip_area_buffers.vertex.slice(..));
                        render_pass.set_index_buffer(
                            clip_area_buffers.index.slice(..),
                            wgpu::IndexFormat::Uint32,
                        );
                        render_pass.draw_indexed(0..clip_area_buffers.index_count, 0, 0..1);
                    }

                    for image in &cast.image_buffers {
                        self.draw_image(&mut render_pass, image);
                    }

                    let buffers = cast.select_poly_buffers(resolution);

                    render_pass.set_pipeline(&self.renderer.line_pipeline);
                    render_pass.set_bind_group(0, &self.renderer.map_view_bind_group, &[]);
                    render_pass.set_vertex_buffer(0, buffers.vertex.slice(..));
                    render_pass
                        .set_index_buffer(buffers.index.slice(..), wgpu::IndexFormat::Uint32);
                    render_pass.draw_indexed(0..buffers.index_count, 0, 0..1);

                    if let Some(point_buffers) = &cast.point_buffers {
                        render_pass.set_pipeline(&self.renderer.point_pipeline);
                        render_pass.set_bind_group(0, &self.renderer.map_view_bind_group, &[]);
                        render_pass.set_vertex_buffer(0, point_buffers.vertex.slice(..));
                        render_pass.set_vertex_buffer(1, point_buffers.instance.slice(..));
                        render_pass.set_index_buffer(
                            point_buffers.index.slice(..),
                            wgpu::IndexFormat::Uint32,
                        );
                        render_pass.draw_indexed(
                            0..point_buffers.index_count,
                            0,
                            0..point_buffers.instance_count,
                        );
                    }

                    if let Some(clip_area_buffers) = &cast.clip_area_buffers {
                        render_pass.set_stencil_reference(0);
                        render_pass.set_pipeline(&self.renderer.clip_pipeline);
                        render_pass.set_bind_group(0, &self.renderer.map_view_bind_group, &[]);
                        render_pass.set_vertex_buffer(0, clip_area_buffers.vertex.slice(..));
                        render_pass.set_index_buffer(
                            clip_area_buffers.index.slice(..),
                            wgpu::IndexFormat::Uint32,
                        );
                        render_pass.draw_indexed(0..clip_area_buffers.index_count, 0, 0..1);
                    }
                }
            }
        }

        self.renderer
            .queue
            .submit(std::iter::once(encoder.finish()));
    }
}

impl<'a> WgpuCanvas<'a> {
    fn draw_image(&self, render_pass: &mut RenderPass<'a>, image: &'a WgpuImage) {
        self.renderer
            .image_painter
            .draw_image(render_pass, image, self.renderer)
    }
}

pub struct WgpuPackedBundle {
    clip_area_buffers: Option<WgpuPolygonBuffers>,
    poly_tessellation: Vec<LodTessellation>,
    poly_buffers: Vec<WgpuPolygonBuffers>,
    point_buffers: Option<WgpuPointBuffers>,
    image_buffers: Vec<WgpuImage>,
    primitives: Vec<PrimitiveInfo>,
}

struct WgpuPolygonBuffers {
    vertex: Buffer,
    index: Buffer,
    index_count: u32,
}

struct WgpuPointBuffers {
    vertex: Buffer,
    index: Buffer,
    instance: Buffer,
    vertices: Vec<PointInstance>,
    index_count: u32,
    instance_count: u32,
}

impl WgpuPackedBundle {
    fn new(
        clip_area: Option<VertexBuffers<PolyVertex, u32>>,
        poly_tessellation: Vec<LodTessellation>,
        points: Vec<PointInstance>,
        images: Vec<(DecodedImage, [ImageVertex; 4])>,
        primitives: Vec<PrimitiveInfo>,
        renderer: &WgpuRenderer,
    ) -> Self {
        let clip_area_buffers = clip_area.map(|v| Self::write_poly_buffers(&v, renderer));

        let mut poly_buffers = vec![];
        for lod in &poly_tessellation {
            poly_buffers.push(Self::write_poly_buffers(&lod.tessellation, renderer));
        }

        let point_buffers = if !points.is_empty() {
            let max_point_size = points.iter().fold(0f32, |v, p| v.max(p.size));
            let (point_indices, point_vertices) = create_point(max_point_size);

            let point_index_buffer =
                renderer
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        contents: bytemuck::cast_slice(&point_indices),
                        usage: wgpu::BufferUsages::INDEX,
                    });

            let point_vertex_buffer =
                renderer
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        usage: wgpu::BufferUsages::VERTEX,
                        contents: bytemuck::cast_slice(&point_vertices),
                    });

            let point_instance_buffer =
                renderer
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        usage: wgpu::BufferUsages::VERTEX,
                        contents: bytemuck::cast_slice(&points),
                    });

            Some(WgpuPointBuffers {
                index: point_index_buffer,
                vertex: point_vertex_buffer,
                instance: point_instance_buffer,
                index_count: point_indices.len() as u32,
                instance_count: points.len() as u32,
                vertices: points,
            })
        } else {
            None
        };

        let mut image_buffers = vec![];
        for (image, vertices) in images {
            let image = renderer.image_painter.create_image(
                &renderer.device,
                &renderer.queue,
                &image,
                vertices,
            );
            image_buffers.push(image);
        }

        Self {
            clip_area_buffers,
            poly_tessellation,
            poly_buffers,
            image_buffers,
            point_buffers,
            primitives,
        }
    }

    fn select_poly_buffers(&self, resolution: f32) -> &WgpuPolygonBuffers {
        for i in 0..self.poly_buffers.len() {
            if self.poly_tessellation[i].min_resolution <= resolution {
                return &self.poly_buffers[i];
            }
        }

        &self.poly_buffers[self.poly_buffers.len() - 1]
    }

    fn write_poly_buffers(
        tessellation: &VertexBuffers<PolyVertex, u32>,
        renderer: &WgpuRenderer,
    ) -> WgpuPolygonBuffers {
        let index = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&tessellation.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        let vertex = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                contents: bytemuck::cast_slice(&tessellation.vertices),
            });

        WgpuPolygonBuffers {
            index,
            vertex,
            index_count: tessellation.indices.len() as u32,
        }
    }
}

fn create_point(max_size: f32) -> (Vec<u32>, Vec<PointVertex>) {
    let half_size = max_size / 2.0;

    let center = PointVertex { norm: [0.0, 0.0] };

    const TOLERANCE: f32 = 0.1;
    let r = half_size.max(1.0);
    let steps_count =
        (std::f32::consts::PI / ((r - TOLERANCE) / (r + TOLERANCE)).acos()).ceil() as usize;

    let mut vertices = vec![center, PointVertex { norm: [1.0, 0.0] }];
    let mut indices = vec![];
    for step in 1..steps_count {
        let angle = 2.0 * std::f32::consts::PI / steps_count as f32 * step as f32;
        let x = angle.cos();
        let y = angle.sin();

        indices.push(0);
        indices.push(vertices.len() as u32 - 1);
        indices.push(vertices.len() as u32);
        vertices.push(PointVertex { norm: [x, y] });
    }

    indices.push(0);
    indices.push(vertices.len() as u32 - 1);
    indices.push(1);

    (indices, vertices)
}

impl PackedBundle for WgpuPackedBundle {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn unpack(self: Box<Self>) -> Box<dyn UnpackedBundle> {
        Box::new(WgpuUnpackedBundle {
            clip_area_buffers: self.clip_area_buffers,
            poly_tessellation: self.poly_tessellation,
            poly_buffers: self.poly_buffers,
            point_buffers: self.point_buffers,
            images: self.image_buffers,
            primitives: self.primitives,
            to_write: Vec::new(),
        })
    }
}

struct WgpuUnpackedBundle {
    clip_area_buffers: Option<WgpuPolygonBuffers>,
    poly_tessellation: Vec<LodTessellation>,
    poly_buffers: Vec<WgpuPolygonBuffers>,
    point_buffers: Option<WgpuPointBuffers>,
    images: Vec<WgpuImage>,
    primitives: Vec<PrimitiveInfo>,

    to_write: Vec<PrimitiveId>,
}

impl WgpuUnpackedBundle {
    fn pack(mut self: Box<Self>, queue: &Queue) -> WgpuPackedBundle {
        self.write_poly_buffers(queue);
        self.write_image_buffers(queue);
        self.write_point_buffers(queue);

        WgpuPackedBundle {
            clip_area_buffers: self.clip_area_buffers,
            poly_tessellation: self.poly_tessellation,
            poly_buffers: self.poly_buffers,
            point_buffers: self.point_buffers,
            image_buffers: self.images,
            primitives: self.primitives,
        }
    }

    fn write_poly_buffers(&mut self, queue: &Queue) {
        for (index, _) in self.poly_tessellation.iter().enumerate() {
            let mut prev: Option<Range<usize>> = None;
            for id in &self.to_write {
                if let PrimitiveInfo::Poly { vertex_ranges } = &self.primitives[id.0] {
                    let vertex_range = &vertex_ranges[index];
                    if let Some(prev_range) = prev {
                        if vertex_range.start == prev_range.end {
                            prev = Some(prev_range.start..vertex_range.end);
                        } else {
                            self.write_buffer_range(index, prev_range, queue);
                            prev = Some(vertex_range.clone());
                        }
                    } else {
                        prev = Some(vertex_range.clone());
                    }
                }
            }

            if let Some(prev_range) = prev {
                self.write_buffer_range(index, prev_range, queue);
            }
        }
    }

    fn write_buffer_range(&self, index: usize, range: Range<usize>, queue: &Queue) {
        queue.write_buffer(
            &self.poly_buffers[index].vertex,
            (range.start * size_of::<PolyVertex>()) as u64,
            bytemuck::cast_slice(&self.poly_tessellation[index].tessellation.vertices[range]),
        );
    }

    fn write_image_buffers(&self, queue: &Queue) {
        for id in &self.to_write {
            if let PrimitiveInfo::Image { image_index } = self.primitives[id.0] {
                let image = &self.images[image_index];
                queue.write_buffer(
                    &image.vertex_buffer,
                    0,
                    bytemuck::cast_slice(&image.vertices[..]),
                )
            }
        }
    }

    fn write_point_buffers(&self, _queue: &Queue) {
        for id in &self.to_write {
            if let PrimitiveInfo::Point { .. } = self.primitives[id.0] {
                todo!()
            }
        }
    }
}

impl UnpackedBundle for WgpuUnpackedBundle {
    fn modify_line(&mut self, id: PrimitiveId, paint: LinePaint) {
        let Some(PrimitiveInfo::Poly { vertex_ranges }) = self.primitives.get(id.0) else {
            return;
        };

        for (index, range) in vertex_ranges.iter().enumerate() {
            for vertex in &mut self.poly_tessellation[index].tessellation.vertices[range.clone()] {
                vertex.color = paint.color.to_f32_array();
            }
        }

        self.to_write.push(id);
    }

    fn modify_polygon(&mut self, id: PrimitiveId, paint: Paint) {
        let Some(PrimitiveInfo::Poly { vertex_ranges }) = self.primitives.get(id.0) else {
            return;
        };

        for (index, range) in vertex_ranges.iter().enumerate() {
            for vertex in &mut self.poly_tessellation[index].tessellation.vertices[range.clone()] {
                vertex.color = paint.color.to_f32_array();
            }
        }

        self.to_write.push(id);
    }

    fn modify_image(&mut self, id: PrimitiveId, paint: ImagePaint) {
        let Some(PrimitiveInfo::Image { image_index }) = self.primitives.get(id.0) else {
            return;
        };

        let image = &mut self.images[*image_index];
        for vertex in image.vertices.iter_mut() {
            vertex.opacity = paint.opacity as f32 / 255.0;
        }

        self.to_write.push(id);
    }

    fn modify_point(&mut self, id: PrimitiveId, paint: PointPaint) {
        let Some(PrimitiveInfo::Point { point_index }) = self.primitives.get(id.0) else {
            return;
        };
        let Some(point_buffers) = &mut self.point_buffers else {
            return;
        };

        let point = &mut point_buffers.vertices[*point_index];
        point.color = paint.color.to_f32_array();
        point.size = paint.size as f32;

        self.to_write.push(id);
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct PointVertex {
    norm: [f32; 2],
}

impl PointVertex {
    fn wgpu_desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<PointVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            }],
        }
    }
}

mod image;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ViewUniform {
    view_proj: [[f32; 4]; 4],
    view_rotation: [[f32; 4]; 4],
    inv_screen_size: [f32; 2],
    resolution: f32,
    _padding: [f32; 1],
}

impl PointInstance {
    fn wgpu_desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<PointInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: (size_of::<[f32; 3]>() + size_of::<f32>()) as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

impl PolyVertex {
    fn wgpu_desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<PolyVertex>() as wgpu::BufferAddress,
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
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: (size_of::<[f32; 3]>() + size_of::<[f32; 4]>()) as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: (size_of::<[f32; 3]>() + size_of::<[f32; 4]>() + size_of::<[f32; 2]>())
                        as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
}
