use galileo_types::cartesian::impls::contour::Contour;
use galileo_types::cartesian::impls::point::Point2d;
use galileo_types::cartesian::impls::polygon::Polygon;
use galileo_types::cartesian::size::Size;
use galileo_types::cartesian::traits::cartesian_point::CartesianPoint2d;
use lyon::lyon_tessellation::{
    BuffersBuilder, FillOptions, FillVertex, LineJoin, Side, StrokeOptions,
};
use lyon::math::point;
use lyon::path::builder::NoAttributes;
use lyon::path::path::BuilderImpl;
use lyon::path::Path;
use lyon::tessellation::{
    FillTessellator, FillVertexConstructor, StrokeTessellator, StrokeVertex,
    StrokeVertexConstructor, VertexBuffers,
};
use nalgebra::Point3;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::mem::size_of;
use std::ops::Range;
use wgpu::util::DeviceExt;
use wgpu::{
    BindGroup, BindGroupLayout, Buffer, Device, Extent3d, Queue, RenderPass, StoreOp,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor,
};
use winit::dpi::PhysicalSize;

use crate::layer::Layer;
use crate::map::Map;
use crate::primitives::{Color, DecodedImage};
use crate::render::wgpu::image::{ImageVertex, WgpuImage};
use crate::view::MapView;

use self::image::ImagePainter;

use super::{
    Canvas, ImagePaint, LinePaint, PackedBundle, Paint, PointPaint, RenderBundle, Renderer,
    UnpackedBundle,
};

pub struct WgpuRenderer {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,

    line_pipeline: wgpu::RenderPipeline,
    image_painter: ImagePainter,
    map_view_buffer: Buffer,
    map_view_bind_group: BindGroup,
    pub map_view_bind_group_layout: BindGroupLayout,
    pub multisampling_view: TextureView,
    pub point_pipeline: wgpu::RenderPipeline,

    background: Color,
}

impl Renderer for WgpuRenderer {
    fn create_bundle(&self) -> Box<dyn RenderBundle> {
        Box::new(WgpuRenderBundle::new())
    }

    fn pack_bundle(&self, bundle: Box<dyn RenderBundle>) -> Box<dyn PackedBundle> {
        let cast = bundle.into_any().downcast::<WgpuRenderBundle>().unwrap();
        Box::new(WgpuPackedBundle::new(
            cast.vertex_buffers,
            cast.ranges,
            cast.points,
            cast.images,
            &self,
        ))
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
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
            size: (std::mem::size_of::<ViewUniform>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        println!("View size is {}", std::mem::size_of::<ViewUniform>());

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

        let line_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Line Render Pipeline"),
            layout: Some(&line_render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &line_shader,
                entry_point: "vs_main",
                buffers: &[LineVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &line_shader,
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
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

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
                buffers: &[PointVertex::desc(), PointInstance::desc()],
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
            depth_stencil: None,
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

        Self {
            surface,
            device,
            queue,
            config,
            size,
            line_pipeline,
            point_pipeline,
            map_view_buffer,
            map_view_bind_group,
            map_view_bind_group_layout,
            image_painter,
            multisampling_view,
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

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            self.multisampling_view =
                Self::create_multisample_texture(&self.device, new_size, self.config.format);
        }
    }

    pub fn render(&self, map: &Map) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor {
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
            self.render_layer(layer, view, texture_view);
        }
    }

    fn render_layer(&self, layer: &Box<dyn Layer>, view: &MapView, texture_view: &TextureView) {
        let mut canvas = WgpuCanvas::new(self, texture_view, view.clone());
        layer.render(&view, &mut canvas);
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
        renderer.queue.write_buffer(
            &renderer.map_view_buffer,
            0,
            bytemuck::cast_slice(&[ViewUniform {
                view_proj: map_view.map_to_scene_mtx().unwrap(),
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

    fn create_bundle(&self) -> Box<dyn RenderBundle> {
        Box::new(WgpuRenderBundle::new())
    }

    fn pack_bundle(&self, bundle: Box<dyn RenderBundle>) -> Box<dyn PackedBundle> {
        let cast = bundle.into_any().downcast::<WgpuRenderBundle>().unwrap();
        Box::new(WgpuPackedBundle::new(
            cast.vertex_buffers,
            cast.ranges,
            cast.points,
            cast.images,
            &self.renderer,
        ))
    }

    fn draw_bundles(&mut self, bundles: &[&Box<dyn PackedBundle>]) {
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
                    resolve_target: Some(&self.view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            for bundle in bundles {
                if let Some(cast) = bundle.as_any().downcast_ref::<WgpuPackedBundle>() {
                    for image in &cast.image_buffers {
                        self.draw_image(&mut render_pass, image);
                    }

                    render_pass.set_pipeline(&self.renderer.line_pipeline);
                    render_pass.set_bind_group(0, &self.renderer.map_view_bind_group, &[]);
                    render_pass.set_vertex_buffer(0, cast.wgpu_buffers.vertex.slice(..));
                    render_pass.set_index_buffer(
                        cast.wgpu_buffers.index.slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    render_pass.draw_indexed(0..cast.index_count(), 0, 0..1);

                    for point_buffers in &cast.point_buffers {
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

#[derive(Debug)]
pub struct WgpuRenderBundle {
    pub vertex_buffers: VertexBuffers<LineVertex, u32>,
    pub ranges: Vec<Range<usize>>,
    points: Vec<(PointPaint, Vec<PointInstance>)>,
    images: Vec<(DecodedImage, [ImageVertex; 4])>,
}

impl WgpuRenderBundle {
    pub fn new() -> Self {
        Self {
            vertex_buffers: VertexBuffers::new(),
            ranges: Vec::new(),
            points: Vec::new(),
            images: Vec::new(),
        }
    }
}

impl RenderBundle for WgpuRenderBundle {
    fn add_image(
        &mut self,
        image: DecodedImage,
        vertices: [Point2d; 4],
        paint: ImagePaint,
    ) -> usize {
        let opacity = paint.opacity as f32 / 255.0;
        self.images.push((
            image,
            [
                ImageVertex {
                    position: [vertices[0].x() as f32, vertices[0].y() as f32],
                    opacity,
                    tex_coords: [0.0, 1.0],
                },
                ImageVertex {
                    position: [vertices[1].x() as f32, vertices[1].y() as f32],
                    opacity,
                    tex_coords: [0.0, 0.0],
                },
                ImageVertex {
                    position: [vertices[3].x() as f32, vertices[3].y() as f32],
                    opacity,
                    tex_coords: [1.0, 1.0],
                },
                ImageVertex {
                    position: [vertices[2].x() as f32, vertices[2].y() as f32],
                    opacity,
                    tex_coords: [1.0, 0.0],
                },
            ],
        ));

        // todo
        0
    }

    fn add_points(&mut self, points: &[Point3<f64>], paint: PointPaint) {
        self.points.push((
            paint,
            points
                .iter()
                .map(|p| PointInstance {
                    position: [p.x as f32, p.y as f32, p.z as f32],
                })
                .collect(),
        ))
    }

    fn add_line(&mut self, line: &Contour<Point2d>, paint: LinePaint, resolution: f64) -> usize {
        let resolution = resolution as f32;
        let vertex_constructor = LineVertexConstructor {
            width: paint.width as f32,
            offset: paint.offset as f32,
            color: paint.color.to_f32_array(),
            resolution,
        };

        // todo: check length of line

        let builder_impl = BuilderImpl::with_capacity(line.points.len(), line.points.len() - 2);
        let mut path_builder = NoAttributes::wrap(builder_impl);

        let _ = path_builder.begin(point(
            line.points[0].x() as f32 / resolution,
            line.points[0].y() as f32 / resolution,
        ));

        for p in line.points.iter().skip(1) {
            let _ =
                path_builder.line_to(point(p.x() as f32 / resolution, p.y() as f32 / resolution));
        }
        let _ = path_builder.end(line.is_closed);
        let path = path_builder.build();

        let mut tesselator = StrokeTessellator::new();
        let start_index = self.vertex_buffers.vertices.len();
        tesselator
            .tessellate(
                &path,
                &StrokeOptions::DEFAULT
                    .with_line_cap(paint.line_cap.into())
                    .with_line_width(paint.width as f32)
                    .with_miter_limit(2.0)
                    .with_tolerance(0.1)
                    .with_line_join(LineJoin::MiterClip),
                &mut BuffersBuilder::new(&mut self.vertex_buffers, vertex_constructor),
            )
            .unwrap();

        let end_index = self.vertex_buffers.vertices.len();
        let id = self.ranges.len();

        self.ranges.push(start_index..end_index);

        id
    }

    fn add_polygon(&mut self, polygon: &Polygon<Point2d>, paint: Paint, _resolution: f64) -> usize {
        let mut path_builder = Path::builder();
        for contour in polygon.iter_contours() {
            let _ = path_builder.begin(point(
                contour.points[0].x() as f32,
                contour.points[0].y() as f32,
            ));

            for p in contour.points.iter().skip(1) {
                let _ = path_builder.line_to(point(p.x() as f32, p.y() as f32));
            }

            let _ = path_builder.end(true);
        }

        let path = path_builder.build();

        let vertex_constructor = PolygonVertexConstructorNew {
            color: paint.color.to_f32_array(),
        };
        let mut tesselator = FillTessellator::new();
        let start_index = self.vertex_buffers.vertices.len();
        tesselator
            .tessellate(
                &path,
                &FillOptions::DEFAULT,
                &mut BuffersBuilder::new(&mut self.vertex_buffers, vertex_constructor),
            )
            .unwrap();

        let end_index = self.vertex_buffers.vertices.len();
        let id = self.ranges.len();

        self.ranges.push(start_index..end_index);

        id
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }
}

pub struct WgpuPackedBundle {
    vertex_buffers: VertexBuffers<LineVertex, u32>,
    ranges: Vec<Range<usize>>,
    wgpu_buffers: WgpuPolygonBuffers,
    point_buffers: Vec<WgpuPointBuffers>,
    image_buffers: Vec<WgpuImage>,
}

struct WgpuPolygonBuffers {
    vertex: Buffer,
    index: Buffer,
}

struct WgpuPointBuffers {
    vertex: Buffer,
    index: Buffer,
    instance: Buffer,
    index_count: u32,
    instance_count: u32,
}

impl WgpuPackedBundle {
    fn new(
        vertex_buffers: VertexBuffers<LineVertex, u32>,
        ranges: Vec<Range<usize>>,
        points: Vec<(PointPaint, Vec<PointInstance>)>,
        images: Vec<(DecodedImage, [ImageVertex; 4])>,
        renderer: &WgpuRenderer,
    ) -> Self {
        let index = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Line index buffer"),
                contents: bytemuck::cast_slice(&vertex_buffers.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        let vertex = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Line vertex buffer"),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                contents: bytemuck::cast_slice(&vertex_buffers.vertices),
            });

        let mut point_buffers = vec![];

        for (paint, point_set) in points {
            let (point_indices, point_vertices) = create_point(paint);
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
                        contents: bytemuck::cast_slice(&point_set),
                    });

            point_buffers.push(WgpuPointBuffers {
                index: point_index_buffer,
                vertex: point_vertex_buffer,
                instance: point_instance_buffer,
                index_count: point_indices.len() as u32,
                instance_count: point_set.len() as u32,
            });
        }

        let mut image_buffers = vec![];
        for (image, vertices) in images {
            let image = renderer.image_painter.create_image(
                &renderer.device,
                &renderer.queue,
                &image,
                &vertices,
            );
            image_buffers.push(image);
        }

        Self {
            vertex_buffers,
            ranges,
            wgpu_buffers: WgpuPolygonBuffers { index, vertex },
            image_buffers,
            point_buffers,
        }
    }

    fn index_count(&self) -> u32 {
        self.vertex_buffers.indices.len() as u32
    }
}

fn create_point(paint: PointPaint) -> (Vec<u32>, Vec<PointVertex>) {
    let size = paint.size;
    let color = paint.color;
    let half_size = size as f32 / 2.0;

    let center = PointVertex {
        position: [0.0, 0.0],
        color: color.to_f32_array(),
    };
    let steps_count = (4 * size as usize).max(16).min(8);
    let color_with_alpha = color.with_alpha(150).to_f32_array();

    let mut vertices = vec![
        center,
        PointVertex {
            position: [half_size, 0.0],
            color: color_with_alpha,
        },
    ];
    let mut indices = vec![];
    for step in 1..steps_count {
        let angle = 2.0 * std::f32::consts::PI / steps_count as f32 * step as f32;
        let x = angle.cos() * half_size;
        let y = angle.sin() * half_size;

        indices.push(0);
        indices.push(vertices.len() as u32 - 1);
        indices.push(vertices.len() as u32);
        vertices.push(PointVertex {
            position: [x, y],
            color: color_with_alpha,
        });
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
            vertex_buffers: self.vertex_buffers,
            ranges: self.ranges,
            wgpu_buffers: self.wgpu_buffers,
            point_buffers: self.point_buffers,
            images: self.image_buffers,
            to_write: Vec::new(),
        })
    }
}

struct WgpuUnpackedBundle {
    vertex_buffers: VertexBuffers<LineVertex, u32>,
    ranges: Vec<Range<usize>>,
    wgpu_buffers: WgpuPolygonBuffers,
    point_buffers: Vec<WgpuPointBuffers>,
    images: Vec<WgpuImage>,

    to_write: Vec<Range<usize>>,
}

impl WgpuUnpackedBundle {
    fn pack(mut self: Box<Self>, queue: &Queue) -> WgpuPackedBundle {
        self.write_buffer(queue);
        WgpuPackedBundle {
            vertex_buffers: self.vertex_buffers,
            ranges: self.ranges,
            wgpu_buffers: self.wgpu_buffers,
            point_buffers: self.point_buffers,
            image_buffers: self.images,
        }
    }
    fn write_buffer(&mut self, queue: &Queue) {
        if self.to_write.is_empty() {
            return;
        }

        let mut prev_range = self.to_write[0].clone();
        for range in &self.to_write[1..] {
            if range.start == prev_range.end {
                prev_range = prev_range.start..range.end;
            } else {
                self.write_buffer_range(prev_range, queue);
                prev_range = range.clone();
            }
        }

        self.write_buffer_range(prev_range, queue);
    }

    fn write_buffer_range(&self, range: Range<usize>, queue: &Queue) {
        queue.write_buffer(
            &self.wgpu_buffers.vertex,
            (range.start * size_of::<LineVertex>()) as u64,
            bytemuck::cast_slice(&self.vertex_buffers.vertices[range]),
        );
    }
}

impl UnpackedBundle for WgpuUnpackedBundle {
    fn modify_line(&mut self, id: usize, paint: LinePaint) {
        let range = self.ranges[id].clone();
        for vertex in &mut self.vertex_buffers.vertices[range.clone()] {
            vertex.color = paint.color.to_f32_array();
        }

        self.to_write.push(range);
    }

    fn modify_polygon(&mut self, id: usize, paint: Paint) {
        let range = self.ranges[id].clone();
        for vertex in &mut self.vertex_buffers.vertices[range.clone()] {
            vertex.color = paint.color.to_f32_array();
        }

        self.to_write.push(range);
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

#[allow(dead_code)]
struct LineVertexConstructor {
    width: f32,
    offset: f32,
    color: [f32; 4],
    resolution: f32,
}

impl StrokeVertexConstructor<LineVertex> for LineVertexConstructor {
    fn new_vertex(&mut self, vertex: StrokeVertex) -> LineVertex {
        let position = vertex.position_on_path();
        let normal = match vertex.side() {
            Side::Negative => [
                vertex.normal().x * (vertex.line_width() - self.offset * 2.0),
                vertex.normal().y * (vertex.line_width() - self.offset * 2.0),
            ],
            Side::Positive => [
                vertex.normal().x * (vertex.line_width() + self.offset * 2.0),
                vertex.normal().y * (vertex.line_width() + self.offset * 2.0),
            ],
        };
        LineVertex {
            position: [position.x * self.resolution, position.y * self.resolution],
            color: self.color,
            normal,
        }
    }
}

struct PolygonVertexConstructor {
    color: [f32; 4],
}

impl FillVertexConstructor<PointVertex> for PolygonVertexConstructor {
    fn new_vertex(&mut self, vertex: FillVertex) -> PointVertex {
        PointVertex {
            position: vertex.position().to_array(),
            color: self.color,
        }
    }
}

struct PolygonVertexConstructorNew {
    color: [f32; 4],
}

impl FillVertexConstructor<LineVertex> for PolygonVertexConstructorNew {
    fn new_vertex(&mut self, vertex: FillVertex) -> LineVertex {
        LineVertex {
            position: vertex.position().to_array(),
            color: self.color,
            normal: Default::default(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize)]
pub struct LineVertex {
    position: [f32; 2],
    color: [f32; 4],
    normal: [f32; 2],
}

impl LineVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<LineVertex>() as wgpu::BufferAddress,
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
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 2]>() + std::mem::size_of::<[f32; 4]>())
                        as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct PointVertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl PointVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<PointVertex>() as wgpu::BufferAddress,
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
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct PointInstance {
    position: [f32; 3],
}

impl PointInstance {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<PointInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x3,
            }],
        }
    }
}

mod image;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ViewUniform {
    view_proj: [[f32; 4]; 4],
    inv_screen_size: [f32; 2],
    resolution: f32,
    _padding: [f32; 1],
}
