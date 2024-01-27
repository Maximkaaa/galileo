use galileo_types::cartesian::size::Size;
use lyon::tessellation::VertexBuffers;
use nalgebra::{Rotation3, Vector3};
use std::any::Any;
use std::mem::size_of;
use std::ops::Range;
use wgpu::util::DeviceExt;
use wgpu::{
    Buffer, Device, Extent3d, Queue, RenderPassDepthStencilAttachment, StoreOp, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
};
use winit::dpi::PhysicalSize;

use crate::layer::Layer;
use crate::map::Map;
use crate::render::point_paint::PointPaint;
use crate::render::render_bundle::tessellating::{
    PointInstance, PolyVertex, PrimitiveInfo, ScreenRefVertex, TessellatingRenderBundle,
};
use crate::render::render_bundle::RenderBundle;
use crate::render::wgpu::pipelines::image::WgpuImage;
use crate::render::wgpu::pipelines::Pipelines;
use crate::view::MapView;
use crate::Color;

use super::{
    Canvas, ImagePaint, LinePaint, PackedBundle, PolygonPaint, PrimitiveId, RenderOptions,
    Renderer, UnpackedBundle,
};

mod pipelines;

const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth24PlusStencil8;

pub struct WgpuRenderer {
    surface: wgpu::Surface,
    device: Device,
    queue: Queue,
    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,
    pipelines: Pipelines,
    multisampling_view: TextureView,
    background: Color,
    stencil_view_multisample: TextureView,
    stencil_view: TextureView,
}

impl Renderer for WgpuRenderer {
    fn create_bundle(&self) -> RenderBundle {
        RenderBundle::Tessellating(TessellatingRenderBundle::new())
    }

    fn pack_bundle(&self, bundle: RenderBundle) -> Box<dyn PackedBundle> {
        match bundle {
            RenderBundle::Tessellating(inner) => Box::new(WgpuPackedBundle::new(inner, self)),
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
                        wgpu::Limits {
                            max_buffer_size: 256 << 22,
                            ..wgpu::Limits::default()
                        }
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
        let multisampling_view = Self::create_multisample_texture(&device, size, config.format);
        let stencil_view_multisample = Self::create_stencil_texture(&device, size, 4);
        let stencil_view = Self::create_stencil_texture(&device, size, 1);

        let pipelines = Pipelines::create(&device, surface_format);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            pipelines,
            multisampling_view,
            stencil_view_multisample,
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

    fn create_stencil_texture(
        device: &Device,
        size: PhysicalSize<u32>,
        sample_count: u32,
    ) -> TextureView {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("Stencil/depth texture"),
            size: Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: TextureDimension::D2,
            format: DEPTH_FORMAT,
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
            self.stencil_view_multisample = Self::create_stencil_texture(&self.device, new_size, 4);
            self.stencil_view = Self::create_stencil_texture(&self.device, new_size, 1);
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
            renderer.pipelines.map_view_buffer(),
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

    fn create_bundle(&self) -> RenderBundle {
        self.renderer.create_bundle()
    }

    fn pack_bundle(&self, bundle: RenderBundle) -> Box<dyn PackedBundle> {
        match bundle {
            RenderBundle::Tessellating(inner) => {
                Box::new(WgpuPackedBundle::new(inner, self.renderer))
            }
        }
    }

    fn pack_unpacked(&self, bundle: Box<dyn UnpackedBundle>) -> Box<dyn PackedBundle> {
        self.renderer.pack_bundle(bundle)
    }

    fn draw_bundles(&mut self, bundles: &[&dyn PackedBundle], options: RenderOptions) {
        let mut encoder =
            self.renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        {
            let (view, resolve_target, depth_view) = if options.antialias {
                (
                    &self.renderer.multisampling_view,
                    Some(self.view),
                    &self.renderer.stencil_view_multisample,
                )
            } else {
                (self.view, None, &self.renderer.stencil_view)
            };

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: StoreOp::Discard,
                    }),
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0),
                        store: StoreOp::Discard,
                    }),
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            for bundle in bundles {
                if let Some(cast) = bundle.as_any().downcast_ref() {
                    self.renderer
                        .pipelines
                        .render(&mut render_pass, cast, options);
                }
            }
        }

        self.renderer
            .queue
            .submit(std::iter::once(encoder.finish()));
    }
}

pub struct WgpuPackedBundle {
    clip_area_buffers: Option<WgpuPolygonBuffers>,
    map_ref_buffers: WgpuPolygonBuffers,
    screen_ref_buffers: Option<ScreenRefBuffers>,
    dot_buffers: Option<WgpuDotBuffers>,
    image_buffers: Vec<WgpuImage>,
    poly_tessellation: VertexBuffers<PolyVertex, u32>,
    primitives: Vec<PrimitiveInfo>,
}

struct WgpuPolygonBuffers {
    vertex: Buffer,
    index: Buffer,
    index_count: u32,
}

struct ScreenRefBuffers {
    vertex: Buffer,
    index: Buffer,
    _vertices: Vec<ScreenRefVertex>,
    index_count: u32,
}

struct WgpuDotBuffers {
    buffer: Buffer,
    _points: Vec<PointInstance>,
    point_count: u32,
}

impl WgpuPackedBundle {
    fn new(bundle: TessellatingRenderBundle, renderer: &WgpuRenderer) -> Self {
        let TessellatingRenderBundle {
            poly_tessellation,
            points,
            screen_ref,
            images,
            primitives,
            clip_area,
        } = bundle;

        let clip_area_buffers = clip_area.map(|v| Self::write_poly_buffers(&v, renderer));

        let poly_buffers = Self::write_poly_buffers(&poly_tessellation, renderer);

        let screen_ref_buffers = if !screen_ref.vertices.is_empty() {
            let index = renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&screen_ref.indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

            let vertex = renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    usage: wgpu::BufferUsages::VERTEX,
                    contents: bytemuck::cast_slice(&screen_ref.vertices),
                });

            Some(ScreenRefBuffers {
                index,
                vertex,
                index_count: screen_ref.indices.len() as u32,
                _vertices: screen_ref.vertices,
            })
        } else {
            None
        };

        let dot_buffers = if points.is_empty() {
            None
        } else {
            let point_instance_buffer =
                renderer
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        usage: wgpu::BufferUsages::VERTEX,
                        contents: bytemuck::cast_slice(&points),
                    });
            let count = points.len();
            Some(WgpuDotBuffers {
                buffer: point_instance_buffer,
                _points: points,
                point_count: count as u32,
            })
        };

        let mut image_buffers = vec![];
        for (image, vertices) in images {
            let image = renderer.pipelines.image_pipeline().create_image(
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
            map_ref_buffers: poly_buffers,
            image_buffers,
            screen_ref_buffers,
            dot_buffers,
            primitives,
        }
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

impl PackedBundle for WgpuPackedBundle {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn unpack(self: Box<Self>) -> Box<dyn UnpackedBundle> {
        Box::new(WgpuUnpackedBundle {
            clip_area_buffers: self.clip_area_buffers,
            poly_tessellation: self.poly_tessellation,
            poly_buffers: self.map_ref_buffers,
            point_buffers: self.screen_ref_buffers,
            images: self.image_buffers,
            dot_buffers: self.dot_buffers,
            primitives: self.primitives,
            to_write: Vec::new(),
        })
    }
}

struct WgpuUnpackedBundle {
    clip_area_buffers: Option<WgpuPolygonBuffers>,
    poly_tessellation: VertexBuffers<PolyVertex, u32>,
    poly_buffers: WgpuPolygonBuffers,
    point_buffers: Option<ScreenRefBuffers>,
    images: Vec<WgpuImage>,
    dot_buffers: Option<WgpuDotBuffers>,
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
            map_ref_buffers: self.poly_buffers,
            screen_ref_buffers: self.point_buffers,
            image_buffers: self.images,
            dot_buffers: self.dot_buffers,
            primitives: self.primitives,
        }
    }

    fn write_poly_buffers(&mut self, queue: &Queue) {
        let mut prev: Option<Range<usize>> = None;
        for id in &self.to_write {
            if let PrimitiveInfo::MapRef { vertex_range } = &self.primitives[id.0] {
                if let Some(prev_range) = prev {
                    if vertex_range.start == prev_range.end {
                        prev = Some(prev_range.start..vertex_range.end);
                    } else {
                        self.write_buffer_range(prev_range, queue);
                        prev = Some(vertex_range.clone());
                    }
                } else {
                    prev = Some(vertex_range.clone());
                }
            }
        }

        if let Some(prev_range) = prev {
            self.write_buffer_range(prev_range, queue);
        }
    }

    fn write_buffer_range(&self, range: Range<usize>, queue: &Queue) {
        queue.write_buffer(
            &self.poly_buffers.vertex,
            (range.start * size_of::<PolyVertex>()) as u64,
            bytemuck::cast_slice(&self.poly_tessellation.vertices[range]),
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
            if let PrimitiveInfo::ScreenRef { .. } = self.primitives[id.0] {
                todo!()
            }
        }
    }
}

impl UnpackedBundle for WgpuUnpackedBundle {
    fn modify_line(&mut self, id: PrimitiveId, paint: LinePaint) {
        let Some(PrimitiveInfo::MapRef { vertex_range }) = self.primitives.get(id.0) else {
            return;
        };

        for vertex in &mut self.poly_tessellation.vertices[vertex_range.clone()] {
            vertex.color = paint.color.to_f32_array();
        }

        self.to_write.push(id);
    }

    fn modify_polygon(&mut self, id: PrimitiveId, paint: PolygonPaint) {
        let Some(PrimitiveInfo::MapRef { vertex_range }) = self.primitives.get(id.0) else {
            return;
        };

        for vertex in &mut self.poly_tessellation.vertices[vertex_range.clone()] {
            vertex.color = paint.color.to_f32_array();
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

    fn modify_point(&mut self, _id: PrimitiveId, _paint: PointPaint) {
        todo!()
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

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
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: (size_of::<[f32; 3]>()) as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Uint8x4,
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
