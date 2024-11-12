use cfg_if::cfg_if;
use galileo_types::cartesian::Size;
use lyon::tessellation::VertexBuffers;
use nalgebra::{Rotation3, Vector3};
use std::any::Any;
use std::mem::size_of;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use wgpu::{
    Adapter, Buffer, BufferAddress, BufferDescriptor, BufferUsages, Device, Extent3d,
    ImageCopyBuffer, ImageCopyTexture, ImageDataLayout, Origin3d, Queue,
    RenderPassDepthStencilAttachment, StoreOp, Surface, SurfaceConfiguration, SurfaceError,
    SurfaceTexture, Texture, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureView, TextureViewDescriptor, WasmNotSendSync,
};

use crate::error::GalileoError;
use crate::layer::Layer;
use crate::map::Map;
use crate::render::render_bundle::tessellating::{
    PointInstance, PolyVertex, TessellatingRenderBundle,
};
use crate::render::render_bundle::{RenderBundle, RenderBundleType};
use crate::render::wgpu::pipelines::image::WgpuImage;
use crate::render::wgpu::pipelines::Pipelines;
use crate::view::MapView;
use crate::Color;

use super::render_bundle::tessellating::{ImageInfo, ImageStoreInfo};
use super::{Canvas, PackedBundle, RenderOptions};

mod pipelines;

const DEFAULT_BACKGROUND: Color = Color::WHITE;
const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth24PlusStencil8;
const TARGET_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;

/// Render backend that uses `wgpu` crate to render the map.
pub struct WgpuRenderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    render_set: Option<RenderSet>,
    background: Color,
}

struct RenderSet {
    render_target: RenderTarget,
    pipelines: Pipelines,
    multisampling_view: TextureView,
    stencil_view_multisample: TextureView,
    stencil_view: TextureView,
}

enum RenderTarget {
    Surface {
        config: SurfaceConfiguration,
        surface: Arc<Surface<'static>>,
    },
    Texture(Texture, Size<u32>),
}

enum RenderTargetTexture<'a> {
    Surface(SurfaceTexture),
    Texture(&'a Texture),
}

impl<'a> RenderTargetTexture<'a> {
    fn view(&self) -> TextureView {
        match self {
            RenderTargetTexture::Surface(t) => t.texture.create_view(&Default::default()),
            RenderTargetTexture::Texture(t) => t.create_view(&Default::default()),
        }
    }

    fn present(self) {
        match self {
            RenderTargetTexture::Surface(t) => t.present(),
            RenderTargetTexture::Texture(_) => {}
        }
    }
}

impl RenderTarget {
    fn texture(&self) -> Result<RenderTargetTexture, SurfaceError> {
        match &self {
            RenderTarget::Surface { surface, .. } => {
                Ok(RenderTargetTexture::Surface(surface.get_current_texture()?))
            }
            RenderTarget::Texture(texture, _) => Ok(RenderTargetTexture::Texture(texture)),
        }
    }

    fn size(&self) -> Size<u32> {
        match &self {
            RenderTarget::Surface { config, .. } => Size::new(config.width, config.height),
            RenderTarget::Texture(_, size) => *size,
        }
    }

    fn format(&self) -> TextureFormat {
        match &self {
            RenderTarget::Surface { config, .. } => config.format,
            RenderTarget::Texture(_, _) => TARGET_TEXTURE_FORMAT,
        }
    }
}

impl WgpuRenderer {
    /// Creates a new wgpu renderer with default parameters.
    ///
    /// Returns `None` if a device adapter cannot be acquired.
    pub async fn new() -> Option<Self> {
        let instance = Self::create_instance();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = Self::create_device(&adapter).await;

        Some(Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            render_set: None,
            background: DEFAULT_BACKGROUND,
        })
    }

    /// Creates a new wgpu renderer that renders the map to an image buffer of the given size.
    ///
    /// Returns `None` if a device adapter cannot be acquired.
    pub async fn new_with_texture_rt(size: Size<u32>) -> Option<Self> {
        let mut renderer = Self::new().await?;
        renderer.init_target_texture(size);

        Some(renderer)
    }

    fn init_target_texture(&mut self, size: Size<u32>) {
        let target_texture = Self::create_target_texture(&self.device, size);
        let render_target = RenderTarget::Texture(target_texture, size);
        self.init_render_set(render_target);
    }

    fn create_target_texture(device: &Device, size: Size<u32>) -> Texture {
        device.create_texture(&TextureDescriptor {
            label: Some("Render target texture"),
            size: Extent3d {
                width: size.width(),
                height: size.height(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TARGET_TEXTURE_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            view_formats: &[],
        })
    }

    fn init_render_set(&mut self, new_target: RenderTarget) {
        let current_set = self.render_set.take();
        match current_set {
            Some(RenderSet {
                render_target,
                pipelines,
                multisampling_view,
                stencil_view_multisample,
                stencil_view,
            }) if new_target.size() == render_target.size() => {
                let pipelines = if new_target.format() == render_target.format() {
                    pipelines
                } else {
                    Pipelines::create(&self.device, new_target.format())
                };

                self.render_set = Some(RenderSet {
                    render_target: new_target,
                    pipelines,
                    multisampling_view,
                    stencil_view_multisample,
                    stencil_view,
                })
            }
            _ => self.render_set = Some(self.create_render_set(new_target)),
        }
    }

    fn create_render_set(&self, render_target: RenderTarget) -> RenderSet {
        let size = render_target.size();
        let format = render_target.format();

        let multisampling_view = Self::create_multisample_texture(&self.device, size, format);
        let stencil_view_multisample = Self::create_stencil_texture(&self.device, size, 4);
        let stencil_view = Self::create_stencil_texture(&self.device, size, 1);

        let pipelines = Pipelines::create(&self.device, format);

        RenderSet {
            render_target,
            pipelines,
            multisampling_view,
            stencil_view_multisample,
            stencil_view,
        }
    }

    /// Creates a new wgpu renderer that renders the map to the given window. The given size must be equal to the
    /// window size.
    ///
    /// Returns `None` if a device adapter cannot be acquired.
    pub async fn new_with_window<W>(window: Arc<W>, size: Size<u32>) -> Option<Self>
    where
        W: raw_window_handle::HasWindowHandle
            + raw_window_handle::HasDisplayHandle
            + WasmNotSendSync
            + 'static,
    {
        let (surface, adapter) = Self::get_window_surface(window).await?;
        let (device, queue) = Self::create_device(&adapter).await;

        let config = Self::get_surface_configuration(&surface, &adapter, size);
        log::info!("Configuring surface with size {size:?}");
        surface.configure(&device, &config);

        Some(Self::new_with_device_and_surface(
            Arc::new(device),
            Arc::new(surface),
            Arc::new(queue),
            config,
        ))
    }

    /// Creates a wgpu surface for the given window.
    ///
    /// Returns `None` if a device adapter cannot be acquired.
    pub async fn get_window_surface<W>(window: Arc<W>) -> Option<(Surface<'static>, Adapter)>
    where
        W: raw_window_handle::HasWindowHandle
            + raw_window_handle::HasDisplayHandle
            + WasmNotSendSync
            + 'static,
    {
        let instance = Self::create_instance();

        log::info!("Creating new surface");
        let surface = match instance.create_surface(window) {
            Ok(s) => s,
            Err(err) => {
                log::warn!("Failed to create a surface from window: {err:?}");
                return None;
            }
        };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;
        Some((surface, adapter))
    }

    fn get_surface_configuration(
        surface: &Surface,
        adapter: &Adapter,
        size: Size<u32>,
    ) -> SurfaceConfiguration {
        let surface_caps = surface.get_capabilities(adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width(),
            height: size.height(),
            present_mode: surface_caps.present_modes[0],
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        }
    }

    /// Creates a new renderer from the initialized wgpu structs.
    pub fn new_with_device_and_surface(
        device: Arc<Device>,
        surface: Arc<Surface<'static>>,
        queue: Arc<Queue>,
        config: SurfaceConfiguration,
    ) -> Self {
        let render_target = RenderTarget::Surface { surface, config };
        let mut renderer = Self {
            device,
            queue,
            render_set: None,
            background: DEFAULT_BACKGROUND,
        };
        renderer.init_render_set(render_target);

        renderer
    }

    /// Set the background color for the map.
    pub fn set_background(&mut self, color: Color) {
        self.background = color;
    }

    /// Returns `true` if the renderer can be used to draw to.
    pub fn initialized(&self) -> bool {
        self.render_set.is_some()
    }

    fn create_instance() -> wgpu::Instance {
        cfg_if! {
            if #[cfg(target_os = "android")] {
                let backends = wgpu::Backends::GL;
            } else {
                let backends = wgpu::Backends::all();
            }
        }

        wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            flags: Default::default(),
            dx12_shader_compiler: Default::default(),
            gles_minor_version: Default::default(),
        })
    }

    async fn create_device(adapter: &Adapter) -> (Device, Queue) {
        adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: if cfg!(any(target_arch = "wasm32", target_os = "android")) {
                        wgpu::Limits {
                            max_texture_dimension_2d: 4096,
                            ..wgpu::Limits::downlevel_webgl2_defaults()
                        }
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .expect("Failed to obtain WGPU device")
    }

    fn create_multisample_texture(
        device: &Device,
        size: Size<u32>,
        format: TextureFormat,
    ) -> TextureView {
        let multisampling_texture = device.create_texture(&TextureDescriptor {
            label: Some("Multisampling texture"),
            size: Extent3d {
                width: size.width(),
                height: size.height(),
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

    fn create_stencil_texture(device: &Device, size: Size<u32>, sample_count: u32) -> TextureView {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("Stencil/depth texture"),
            size: Extent3d {
                width: size.width(),
                height: size.height(),
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

    /// De-initializes the renderer.
    pub fn clear_render_target(&mut self) {
        self.render_set = None;
    }

    /// Re-initializes the renderer with the given window.
    ///
    /// Returns an error if a device adapter cannot be acquired.
    pub async fn init_with_window<W>(
        &mut self,
        window: Arc<W>,
        size: Size<u32>,
    ) -> Result<(), GalileoError>
    where
        W: raw_window_handle::HasWindowHandle
            + raw_window_handle::HasDisplayHandle
            + WasmNotSendSync
            + 'static,
    {
        let Some((surface, adapter)) = Self::get_window_surface(window).await else {
            return Err(GalileoError::Generic("Failed to create surface".into()));
        };
        self.init_with_surface(surface, adapter, size);

        Ok(())
    }

    /// Re-initializes the renderer with the given surface and adapter.
    pub fn init_with_surface(
        &mut self,
        surface: Surface<'static>,
        adapter: Adapter,
        size: Size<u32>,
    ) {
        let config = Self::get_surface_configuration(&surface, &adapter, size);
        surface.configure(&self.device, &config);

        let render_target = RenderTarget::Surface {
            surface: Arc::new(surface),
            config,
        };
        self.init_render_set(render_target);
    }

    /// Changes the size of the buffer to be drawn to.
    ///
    /// This must be called if a window size is change before any render calls are done.
    pub fn resize(&mut self, new_size: Size<u32>) {
        let format = self.target_format();
        let Some(render_set) = &mut self.render_set else {
            return;
        };

        if render_set.render_target.size() != new_size
            && new_size.width() > 0
            && new_size.height() > 0
        {
            match &mut render_set.render_target {
                RenderTarget::Surface { config, surface } => {
                    config.width = new_size.width();
                    config.height = new_size.height();
                    log::info!("Configuring surface with size {new_size:?}");
                    surface.configure(&self.device, config);
                }
                RenderTarget::Texture(texture, size) => {
                    *texture = Self::create_target_texture(&self.device, *size);
                    *size = new_size
                }
            }

            render_set.multisampling_view =
                Self::create_multisample_texture(&self.device, new_size, format);
            render_set.stencil_view_multisample =
                Self::create_stencil_texture(&self.device, new_size, 4);
            render_set.stencil_view = Self::create_stencil_texture(&self.device, new_size, 1);
        }
    }

    fn target_format(&self) -> TextureFormat {
        match &self.render_set {
            Some(RenderSet {
                render_target: RenderTarget::Surface { config, .. },
                ..
            }) => config.format,
            _ => TARGET_TEXTURE_FORMAT,
        }
    }

    /// Returns the image of the last render operation.
    pub async fn get_image(&self) -> Result<Vec<u8>, SurfaceError> {
        let Some(render_set) = &self.render_set else {
            return Err(SurfaceError::Lost);
        };

        let size = render_set.render_target.size();
        let buffer_size = (size.width() * size.height() * size_of::<u32>() as u32) as BufferAddress;
        let buffer_desc = BufferDescriptor {
            size: buffer_size,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            label: None,
            mapped_at_creation: false,
        };
        let buffer = self.device.create_buffer(&buffer_desc);

        let RenderTarget::Texture(texture, _) = &render_set.render_target else {
            todo!()
        };

        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            ImageCopyTexture {
                aspect: TextureAspect::All,
                texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
            },
            ImageCopyBuffer {
                buffer: &buffer,
                layout: ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(size_of::<u32>() as u32 * size.width()),
                    rows_per_image: Some(size.height()),
                },
            },
            Extent3d {
                width: size.width(),
                height: size.height(),
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(Some(encoder.finish()));

        let buffer_slice = buffer.slice(..);
        let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            if let Err(err) = tx.send(result) {
                log::error!("Failed to send by channel: {err:?}");
            }
        });
        self.device.poll(wgpu::Maintain::Wait);
        match rx.receive().await {
            Some(result) => match result {
                Ok(()) => {}
                Err(err) => {
                    log::error!("Writing to image buffer failed: {err:?}.");
                    return Err(SurfaceError::Lost);
                }
            },
            None => {
                log::error!("Channel was closed");
                return Err(SurfaceError::Lost);
            }
        }

        let data = buffer_slice.get_mapped_range();
        Ok(data.to_vec())
    }

    /// Renders the map to the given texture.
    pub fn render_to_texture_view(&self, map: &Map, view: &TextureView) {
        if let Some(render_set) = &self.render_set {
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
                        view: &render_set.multisampling_view,
                        resolve_target: Some(view),
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
        } else {
            return;
        }

        self.render_map(map, view);
    }

    /// Renders the map.
    pub fn render(&self, map: &Map) -> Result<(), SurfaceError> {
        let Some(render_set) = &self.render_set else {
            return Ok(());
        };

        let texture = render_set.render_target.texture()?;
        let view = texture.view();

        self.render_to_texture_view(map, &view);

        texture.present();

        Ok(())
    }

    fn render_map(&self, map: &Map, texture_view: &TextureView) {
        let view = map.view();
        for layer in map.layers().iter_visible() {
            self.render_layer(layer, view, texture_view);
        }
    }

    fn render_layer(&self, layer: &dyn Layer, view: &MapView, texture_view: &TextureView) {
        let Some(render_set) = &self.render_set else {
            return;
        };
        let Some(mut canvas) = WgpuCanvas::new(self, render_set, texture_view, view.clone()) else {
            log::warn!("Layer cannot be rendered to the map view.");
            return;
        };

        layer.render(view, &mut canvas);
    }

    /// Returns the size of the rendering area.
    pub fn size(&self) -> Size {
        let size = match &self.render_set {
            Some(set) => set.render_target.size(),
            None => Size::default(),
        };

        Size::new(size.width() as f64, size.height() as f64)
    }

    fn create_bundle(&self) -> RenderBundle {
        RenderBundle(RenderBundleType::Tessellating(
            TessellatingRenderBundle::new(),
        ))
    }
}

#[allow(dead_code)]
struct WgpuCanvas<'a> {
    renderer: &'a WgpuRenderer,
    render_set: &'a RenderSet,
    view: &'a TextureView,
}

impl<'a> WgpuCanvas<'a> {
    fn new(
        renderer: &'a WgpuRenderer,
        render_set: &'a RenderSet,
        view: &'a TextureView,
        map_view: MapView,
    ) -> Option<Self> {
        let rotation_mtx = Rotation3::new(Vector3::new(
            map_view.rotation_x(),
            0.0,
            -map_view.rotation_z(),
        ))
        .to_homogeneous();
        renderer.queue.write_buffer(
            render_set.pipelines.map_view_buffer(),
            0,
            bytemuck::cast_slice(&[ViewUniform {
                view_proj: map_view.map_to_scene_mtx()?,
                view_rotation: rotation_mtx.cast::<f32>().data.0,
                inv_screen_size: [
                    1.0 / renderer.size().width() as f32,
                    1.0 / renderer.size().height() as f32,
                ],
                resolution: map_view.resolution() as f32,
                _padding: [0.0; 1],
            }]),
        );

        Some(Self {
            renderer,
            render_set,
            view,
        })
    }
}

impl<'a> Canvas for WgpuCanvas<'a> {
    fn size(&self) -> Size {
        self.renderer.size()
    }

    fn create_bundle(&self) -> RenderBundle {
        self.renderer.create_bundle()
    }

    fn pack_bundle(&self, bundle: &RenderBundle) -> Box<dyn PackedBundle> {
        match bundle {
            RenderBundle(RenderBundleType::Tessellating(inner)) => {
                Box::new(WgpuPackedBundle::new(inner, self.renderer, self.render_set))
            }
        }
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
                    &self.render_set.multisampling_view,
                    Some(self.view),
                    &self.render_set.stencil_view_multisample,
                )
            } else {
                (self.view, None, &self.render_set.stencil_view)
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
                if let Some(cast) = bundle.as_any().downcast_ref::<WgpuPackedBundle>() {
                    self.render_set
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

struct WgpuPackedBundle {
    clip_area_buffers: Option<WgpuPolygonBuffers>,
    map_ref_buffers: WgpuPolygonBuffers,
    screen_ref_buffers: Option<ScreenRefBuffers>,
    dot_buffers: Option<WgpuDotBuffers>,
    image_buffers: Vec<WgpuImage>,
}

struct WgpuPolygonBuffers {
    vertex: Buffer,
    index: Buffer,
    index_count: u32,
}

struct ScreenRefBuffers {
    vertex: Buffer,
    index: Buffer,
    index_count: u32,
}

struct WgpuDotBuffers {
    buffer: Buffer,
    point_count: u32,
}

impl WgpuPackedBundle {
    fn new(
        bundle: &TessellatingRenderBundle,
        renderer: &WgpuRenderer,
        render_set: &RenderSet,
    ) -> Self {
        let TessellatingRenderBundle {
            poly_tessellation,
            points,
            screen_ref,
            images,
            clip_area,
            image_store,
            ..
        } = bundle;

        let clip_area_buffers = clip_area
            .as_ref()
            .map(|v| Self::write_poly_buffers(v, renderer));

        let poly_buffers = Self::write_poly_buffers(poly_tessellation, renderer);

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
                        contents: bytemuck::cast_slice(points),
                    });
            let count = points.len();
            Some(WgpuDotBuffers {
                buffer: point_instance_buffer,
                point_count: count as u32,
            })
        };

        let textures: Vec<_> = image_store
            .iter()
            .map(|stored| match stored {
                ImageStoreInfo::Vacant => None,
                ImageStoreInfo::Image(decoded_image) => {
                    Some(render_set.pipelines.image_pipeline().create_image_texture(
                        &renderer.device,
                        &renderer.queue,
                        decoded_image,
                    ))
                }
            })
            .collect();

        let mut image_buffers = vec![];
        for image_info in images {
            if let ImageInfo::Image((image_index, vertices)) = image_info {
                let image = render_set.pipelines.image_pipeline().create_image(
                    &renderer.device,
                    textures
                        .get(*image_index)
                        .expect("texture at index must exist")
                        .clone()
                        .expect("image texture must not be None")
                        .clone(),
                    vertices,
                );
                image_buffers.push(image);
            } else {
                // ignore vacant image slots
            }
        }

        Self {
            clip_area_buffers,
            map_ref_buffers: poly_buffers,
            image_buffers,
            screen_ref_buffers,
            dot_buffers,
        }
    }

    fn write_poly_buffers(
        tessellation: &VertexBuffers<PolyVertex, u32>,
        renderer: &WgpuRenderer,
    ) -> WgpuPolygonBuffers {
        let index_bytes = bytemuck::cast_slice(&tessellation.indices);
        let bytes = bytemuck::cast_slice(&tessellation.vertices);

        let index = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: index_bytes,
                usage: wgpu::BufferUsages::INDEX,
            });

        let vertex = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                contents: bytes,
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
