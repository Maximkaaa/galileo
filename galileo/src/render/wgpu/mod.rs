use std::any::Any;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::mem::size_of;
use std::sync::{Arc, Weak};
use std::time::Duration;

use ahash::HashMap;
use cfg_if::cfg_if;
use effects::horizon::HorizonPipeline;
use galileo_types::cartesian::{Rect, Size};
use lyon::tessellation::VertexBuffers;
use nalgebra::{Point4, Rotation3, Vector3};
use parking_lot::Mutex;
use wgpu::util::DeviceExt;
use wgpu::{
    Adapter, BindGroup, Buffer, BufferAddress, BufferDescriptor, BufferUsages, Device, Extent3d,
    Origin3d, Queue, RenderPassDepthStencilAttachment, StoreOp, Surface, SurfaceConfiguration,
    SurfaceError, SurfaceTexture, TexelCopyBufferInfo, TexelCopyBufferLayout, TexelCopyTextureInfo,
    Texture, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureView, TextureViewDescriptor, WasmNotSendSync,
};

use super::render_bundle::screen_set::{RenderSetState, ScreenSetData};
use super::{Canvas, PackedBundle, RenderOptions};
use crate::decoded_image::DecodedImage;
use crate::error::GalileoError;
use crate::map::Map;
use crate::render::render_bundle::world_set::{PointInstance, PolyVertex, WorldRenderSet};
use crate::render::render_bundle::RenderBundle;
use crate::render::wgpu::pipelines::image::WgpuImage;
use crate::render::wgpu::pipelines::Pipelines;
use crate::view::MapView;
use crate::Color;

mod effects;
mod pipelines;

pub use effects::horizon::HorizonOptions;

const DEFAULT_BACKGROUND: Color = Color::WHITE;
const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth24PlusStencil8;
const TARGET_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;

type TexturesMap = HashMap<u64, (Weak<DecodedImage>, Arc<BindGroup>)>;

/// Render backend that uses `wgpu` crate to render the map.
pub struct WgpuRenderer {
    device: Device,
    queue: Queue,
    renderer_targets: Option<RendererTargets>,
    background: Color,
    textures: Mutex<TexturesMap>,
    horizon_options: Option<HorizonOptions>,
}

struct RendererTargets {
    render_target: RenderTarget,
    pipelines: Pipelines,
    multisampling_view: TextureView,
    stencil_view_multisample: TextureView,
    stencil_view: TextureView,
    horizon_effect: Option<HorizonPipeline>,
}

enum RenderTarget {
    Surface {
        config: SurfaceConfiguration,
        surface: Surface<'static>,
    },
    Texture(Texture, Size<u32>),
}

enum RenderTargetTexture<'a> {
    Surface(SurfaceTexture),
    Texture(&'a Texture),
}

impl RenderTargetTexture<'_> {
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
    fn texture(&self) -> Result<RenderTargetTexture<'_>, SurfaceError> {
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
            .await
            .ok()?;

        let (device, queue) = Self::create_device(&adapter).await;

        Some(Self {
            device,
            queue,
            renderer_targets: None,
            background: DEFAULT_BACKGROUND,
            textures: Default::default(),
            horizon_options: Some(HorizonOptions::default()),
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
        self.init_renderer_targets(render_target);
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
            usage: TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::COPY_SRC
                | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    }

    fn init_renderer_targets(&mut self, new_target: RenderTarget) {
        let current_set = self.renderer_targets.take();
        match current_set {
            Some(RendererTargets {
                render_target,
                pipelines,
                multisampling_view,
                stencil_view_multisample,
                stencil_view,
                horizon_effect,
            }) if new_target.size() == render_target.size() => {
                let pipelines = if new_target.format() == render_target.format() {
                    pipelines
                } else {
                    Pipelines::create(&self.device, new_target.format())
                };

                self.renderer_targets = Some(RendererTargets {
                    render_target: new_target,
                    pipelines,
                    multisampling_view,
                    stencil_view_multisample,
                    stencil_view,
                    horizon_effect,
                })
            }
            _ => self.renderer_targets = Some(self.create_renderer_targets(new_target)),
        }
    }

    fn create_renderer_targets(&self, render_target: RenderTarget) -> RendererTargets {
        let size = render_target.size();
        let format = render_target.format();

        let multisampling_view = Self::create_multisample_texture(&self.device, size, format);
        let stencil_view_multisample = Self::create_stencil_texture(&self.device, size, 4);
        let stencil_view = Self::create_stencil_texture(&self.device, size, 1);

        let pipelines = Pipelines::create(&self.device, format);

        let horizon_effect = self.horizon_options.map(|options| {
            HorizonPipeline::create(
                &self.device,
                format,
                &pipelines.map_view_bind_group_layout,
                options,
            )
        });

        RendererTargets {
            render_target,
            pipelines,
            multisampling_view,
            stencil_view_multisample,
            stencil_view,
            horizon_effect,
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
            device, surface, queue, config,
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
            .await
            .ok()?;
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
        device: Device,
        surface: Surface<'static>,
        queue: Queue,
        config: SurfaceConfiguration,
    ) -> Self {
        let render_target = RenderTarget::Surface { surface, config };
        let mut renderer = Self {
            device,
            queue,
            renderer_targets: None,
            background: DEFAULT_BACKGROUND,
            textures: Default::default(),
            horizon_options: Some(HorizonOptions::default()),
        };
        renderer.init_renderer_targets(render_target);

        renderer
    }

    /// Creates a new rendering using the given wgpu device and queue, and rendering to a texture
    /// with the given size.
    pub fn new_with_device_and_texture(device: Device, queue: Queue, size: Size<u32>) -> Self {
        let mut renderer = Self {
            device,
            queue,
            renderer_targets: None,
            background: DEFAULT_BACKGROUND,
            textures: Default::default(),
            horizon_options: Some(HorizonOptions::default()),
        };

        renderer.init_target_texture(size);

        renderer
    }

    /// Set the background color for the map.
    pub fn set_background(&mut self, color: Color) {
        self.background = color;
    }

    /// Returns `true` if the renderer can be used to draw to.
    pub fn initialized(&self) -> bool {
        self.renderer_targets.is_some()
    }

    fn create_instance() -> wgpu::Instance {
        cfg_if! {
            if #[cfg(target_os = "android")] {
                let backends = wgpu::Backends::GL;
            } else {
                let backends = wgpu::Backends::all();
            }
        }

        wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        })
    }

    async fn create_device(adapter: &Adapter) -> (Device, Queue) {
        adapter
            .request_device(&wgpu::DeviceDescriptor {
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
                trace: wgpu::Trace::Off,
            })
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
        self.renderer_targets = None;
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

        let render_target = RenderTarget::Surface { surface, config };
        self.init_renderer_targets(render_target);
    }

    /// Changes the size of the buffer to be drawn to.
    ///
    /// This must be called if a window size is change before any render calls are done.
    pub fn resize(&mut self, new_size: Size<u32>) {
        let format = self.target_format();
        let Some(renderer_targets) = &mut self.renderer_targets else {
            return;
        };

        if renderer_targets.render_target.size() != new_size
            && new_size.width() > 0
            && new_size.height() > 0
        {
            match &mut renderer_targets.render_target {
                RenderTarget::Surface { config, surface } => {
                    config.width = new_size.width();
                    config.height = new_size.height();
                    log::info!("Configuring surface with size {new_size:?}");
                    surface.configure(&self.device, config);
                }
                RenderTarget::Texture(texture, size) => {
                    *texture = Self::create_target_texture(&self.device, new_size);
                    *size = new_size
                }
            }

            renderer_targets.multisampling_view =
                Self::create_multisample_texture(&self.device, new_size, format);
            renderer_targets.stencil_view_multisample =
                Self::create_stencil_texture(&self.device, new_size, 4);
            renderer_targets.stencil_view = Self::create_stencil_texture(&self.device, new_size, 1);
        }
    }

    fn target_format(&self) -> TextureFormat {
        match &self.renderer_targets {
            Some(RendererTargets {
                render_target: RenderTarget::Surface { config, .. },
                ..
            }) => config.format,
            _ => TARGET_TEXTURE_FORMAT,
        }
    }

    /// Returns target texture view.
    ///
    /// Returns `None` if render target is not initialized.
    pub fn get_target_texture_view(&self) -> Option<TextureView> {
        self.renderer_targets
            .as_ref()
            .and_then(|rs| rs.render_target.texture().ok().map(|rt| rt.view()))
    }

    /// Returns the image of the last render operation.
    pub async fn get_image(&self) -> Result<Vec<u8>, SurfaceError> {
        let Some(renderer_targets) = &self.renderer_targets else {
            return Err(SurfaceError::Lost);
        };

        let size = renderer_targets.render_target.size();
        let buffer_size = (size.width() * size.height() * size_of::<u32>() as u32) as BufferAddress;
        let buffer_desc = BufferDescriptor {
            size: buffer_size,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            label: None,
            mapped_at_creation: false,
        };
        let buffer = self.device.create_buffer(&buffer_desc);

        let RenderTarget::Texture(texture, _) = &renderer_targets.render_target else {
            todo!()
        };

        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            TexelCopyTextureInfo {
                aspect: TextureAspect::All,
                texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
            },
            TexelCopyBufferInfo {
                buffer: &buffer,
                layout: TexelCopyBufferLayout {
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

        if let Err(err) = self.device.poll(wgpu::PollType::Wait) {
            log::error!("polling device failed: {err:?}");
        }

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
        if let Some(renderer_targets) = &self.renderer_targets {
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
                        view: &renderer_targets.multisampling_view,
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
        let Some(renderer_targets) = &self.renderer_targets else {
            return Ok(());
        };

        self.trim_textures();

        let texture = renderer_targets.render_target.texture()?;
        let view = texture.view();

        self.render_to_texture_view(map, &view);

        texture.present();

        Ok(())
    }

    fn render_map(&self, map: &Map, texture_view: &TextureView) {
        let view = map.view();
        let Some(renderer_targets) = &self.renderer_targets else {
            return;
        };

        let Some(mut canvas) = WgpuCanvas::new(self, renderer_targets, texture_view, view.clone())
        else {
            log::warn!("Layer cannot be rendered to the map view.");
            return;
        };

        for layer in map.layers().iter_visible() {
            layer.prepare(view, &mut canvas);
            layer.render(view, &mut canvas);
        }

        let needs_animation = canvas.draw_screen_sets();
        if needs_animation {
            map.redraw();
        }

        self.draw_horizon(view, renderer_targets, texture_view);
    }

    /// Returns options of the horizon effect used by the renderer.
    pub fn horizon_options(&self) -> &Option<HorizonOptions> {
        &self.horizon_options
    }

    /// Updates the options of the horizon effect.
    ///
    /// If `None` is given, the effect will not be used.
    pub fn set_horizon_options(&mut self, options: Option<HorizonOptions>) {
        self.horizon_options = options;
        if let Some(targets) = &mut self.renderer_targets {
            targets.horizon_effect = options.map(|op| {
                HorizonPipeline::create(
                    &self.device,
                    targets.render_target.format(),
                    &targets.pipelines.map_view_bind_group_layout,
                    op,
                )
            });
        }
    }

    fn draw_horizon(
        &self,
        view: &MapView,
        renderer_targets: &RendererTargets,
        texture_view: &TextureView,
    ) {
        let Some(pipeline) = &renderer_targets.horizon_effect else {
            return;
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Horizon Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &renderer_targets.stencil_view,
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

            renderer_targets.pipelines.set_bindings(&mut render_pass);
            pipeline.render(view, &self.queue, &mut render_pass, 0);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Returns the size of the rendering area.
    pub fn size(&self) -> Size {
        let size = match &self.renderer_targets {
            Some(set) => set.render_target.size(),
            None => Size::default(),
        };

        Size::new(size.width() as f64, size.height() as f64)
    }

    /// Deallocates all unneeded textures.
    ///
    /// A texture is considered unneeded if there are no rereferences left to the underlying image.
    pub fn trim_textures(&self) {
        self.textures
            .lock()
            .retain(|_, (image_ref, _)| image_ref.strong_count() > 0);
    }

    fn get_or_create_image_texture(&self, image: &Arc<DecodedImage>) -> Arc<BindGroup> {
        let mut hasher = ahash::AHasher::default();
        image.hash(&mut hasher);
        let hash = hasher.finish();

        if let Some((_, texture)) = self.textures.lock().get(&hash) {
            return texture.clone();
        }

        let texture = self
            .renderer_targets
            .as_ref()
            .expect("trying to use pipelines of uninitialized renderer")
            .pipelines
            .create_image_texture(&self.device, &self.queue, image);

        self.textures
            .lock()
            .insert(hash, (Arc::downgrade(image), texture.clone()));

        texture
    }
}

struct WgpuCanvas<'a> {
    renderer: &'a WgpuRenderer,
    renderer_targets: &'a RendererTargets,
    view: &'a TextureView,
    map_view: MapView,

    screen_sets: Vec<Arc<Mutex<WgpuScreenSet>>>,
}

impl<'a> WgpuCanvas<'a> {
    fn new(
        renderer: &'a WgpuRenderer,
        renderer_targets: &'a RendererTargets,
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
            renderer_targets.pipelines.map_view_buffer(),
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
            renderer_targets,
            view,
            map_view,
            screen_sets: vec![],
        })
    }
}

impl Canvas for WgpuCanvas<'_> {
    fn size(&self) -> Size {
        self.renderer.size()
    }

    fn dpi_scale_factor(&self) -> f32 {
        self.map_view.dpi_scale_factor()
    }

    fn pack_bundle(&self, bundle: &RenderBundle) -> Box<dyn PackedBundle> {
        Box::new(WgpuPackedBundle::new(
            bundle,
            self.renderer,
            self.renderer_targets,
            self.map_view.dpi_scale_factor(),
        ))
    }

    fn draw_bundles(&mut self, bundles: &[&dyn PackedBundle], options: RenderOptions) {
        let with_opacity: Vec<_> = bundles.iter().map(|bundle| (*bundle, 1.0)).collect();
        self.draw_bundles_with_opacity(&with_opacity, options);
    }

    fn draw_bundles_with_opacity(
        &mut self,
        bundles: &[(&dyn PackedBundle, f32)],
        options: RenderOptions,
    ) {
        if bundles.is_empty() {
            log::debug!("Requested drawing of 0 bundles");
            return;
        }

        let mut encoder =
            self.renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        {
            let (view, resolve_target, depth_view) = if options.antialias {
                (
                    &self.renderer_targets.multisampling_view,
                    Some(self.view),
                    &self.renderer_targets.stencil_view_multisample,
                )
            } else {
                (self.view, None, &self.renderer_targets.stencil_view)
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

            let opacities: Vec<f32> = bundles.iter().map(|(_, opacity)| *opacity).collect();
            let display_buffer =
                self.renderer
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        usage: wgpu::BufferUsages::VERTEX,
                        contents: bytemuck::cast_slice(&opacities),
                    });
            render_pass.set_vertex_buffer(1, display_buffer.slice(..));

            for (index, (bundle, _)) in bundles.iter().enumerate() {
                if let Some(cast) = bundle.as_any().downcast_ref::<WgpuPackedBundle>() {
                    self.renderer_targets.pipelines.render(
                        &mut render_pass,
                        cast,
                        options,
                        index as u32,
                    );

                    for screen_set in &cast.screen_sets {
                        self.screen_sets.push(screen_set.clone());
                    }
                }
            }
        }

        self.renderer
            .queue
            .submit(std::iter::once(encoder.finish()));
    }

    fn draw_screen_sets(&mut self) -> bool {
        if self.screen_sets.is_empty() {
            return false;
        }

        let view = &self.map_view;
        let Some(transform) = view.map_to_scene_transform() else {
            // current view cannot be rendered to screen
            return false;
        };
        let size = view.size();

        let screen_sets = std::mem::take(&mut self.screen_sets);
        let mut sets: Vec<_> = screen_sets
            .iter()
            .map(|set| {
                let locked = set.lock();
                let projected_anchor = transform
                    * Point4::new(
                        locked.anchor_point[0] as f64,
                        locked.anchor_point[1] as f64,
                        locked.anchor_point[2] as f64,
                        1.0,
                    );
                let normalaized = projected_anchor / projected_anchor.w.abs();

                (locked, normalaized)
            })
            .collect();
        sets.sort_by(|a, b| {
            let displayed_cmp = match (a.0.state.is_displayed(), b.0.state.is_displayed()) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => Ordering::Equal,
            };

            displayed_cmp.then(
                a.1.z
                    .partial_cmp(&b.1.z)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
        });

        let now = web_time::Instant::now();
        let mut displayed: Vec<Rect<f32>> = vec![];
        let mut filtered_sets: Vec<_> = sets
            .into_iter()
            .filter_map(|(mut set, anchor)| {
                if anchor.w <= 0.0 {
                    // The point is in imaginary plane
                    return None;
                }

                let dx = anchor.x * size.width() / 2.0;
                let dy = anchor.y * size.height() / 2.0;

                let set_bbox = set.bbox.shift(dx as f32, dy as f32);

                if set.hide_on_overlay && displayed.iter().any(|bbox| bbox.intersects(set_bbox)) {
                    // Hiding the set
                    match set.state {
                        RenderSetState::Hidden => None,
                        RenderSetState::FadingIn { start_time } => {
                            let fade_out_start_time =
                                now + (now - start_time) - set.animation_duration;
                            set.state = RenderSetState::FadingOut {
                                start_time: fade_out_start_time,
                            };

                            Some(set)
                        }
                        RenderSetState::Displayed => {
                            set.state = RenderSetState::FadingOut {
                                start_time: web_time::Instant::now(),
                            };
                            Some(set)
                        }
                        RenderSetState::FadingOut { .. } => Some(set),
                    }
                } else {
                    // Showing the set
                    displayed.push(set_bbox);

                    match set.state {
                        RenderSetState::Hidden => {
                            set.state = RenderSetState::FadingIn {
                                start_time: web_time::Instant::now(),
                            };
                        }
                        RenderSetState::FadingOut { start_time } => {
                            let fade_in_start_time =
                                now + (now - start_time) - set.animation_duration;
                            set.state = RenderSetState::FadingIn {
                                start_time: fade_in_start_time,
                            };
                        }
                        _ => {}
                    }

                    Some(set)
                }
            })
            .collect();

        let mut is_animating = false;
        let mut encoder =
            self.renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        {
            let view = &self.renderer_targets.multisampling_view;
            let resolve_target = Some(self.view);
            let depth_view = &self.renderer_targets.stencil_view_multisample;

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

            let instances: Vec<ScreenSetInstance> = filtered_sets
                .iter_mut()
                .map(|set| {
                    let opacity = match set.state {
                        RenderSetState::Hidden => 0.0,
                        RenderSetState::FadingIn { start_time } => {
                            is_animating = true;

                            let mut opacity = if set.animation_duration.is_zero() {
                                1.0
                            } else {
                                (now - start_time).as_millis() as f32
                                    / set.animation_duration.as_millis() as f32
                            };

                            if opacity >= 1.0 {
                                set.state = RenderSetState::Displayed;
                                opacity = 1.0;
                            }

                            opacity
                        }
                        RenderSetState::Displayed => 1.0,
                        RenderSetState::FadingOut { start_time } => {
                            is_animating = true;

                            let mut opacity = if set.animation_duration.is_zero() {
                                0.0
                            } else {
                                1.0 - ((now - start_time).as_millis() as f32
                                    / set.animation_duration.as_millis() as f32)
                            };

                            if opacity <= 0.0 {
                                set.state = RenderSetState::Hidden;
                                opacity = 0.0;
                            }

                            opacity
                        }
                    };
                    ScreenSetInstance {
                        anchor: set.anchor_point,
                        opacity,
                    }
                })
                .collect();

            let display_buffer =
                self.renderer
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        usage: wgpu::BufferUsages::VERTEX,
                        contents: bytemuck::cast_slice(&instances),
                    });

            render_pass.set_vertex_buffer(1, display_buffer.slice(..));

            for (index, set) in filtered_sets.iter().enumerate().rev() {
                self.renderer_targets.pipelines.render_screen_set(
                    &set.data,
                    &mut render_pass,
                    index as u32,
                );
            }
        }

        self.renderer
            .queue
            .submit(std::iter::once(encoder.finish()));

        is_animating
    }
}

struct WgpuPackedBundle {
    clip_area_buffers: Option<WgpuVertexBuffers>,
    map_ref_buffers: WgpuVertexBuffers,
    dot_buffers: Option<WgpuDotBuffers>,
    image_buffers: Vec<WgpuImage>,

    screen_sets: Vec<Arc<Mutex<WgpuScreenSet>>>,
}

struct WgpuScreenSet {
    state: RenderSetState,
    animation_duration: Duration,
    anchor_point: [f32; 3],
    bbox: Rect<f32>,
    hide_on_overlay: bool,
    data: WgpuScreenSetData,
}

enum WgpuScreenSetData {
    Vertex(WgpuVertexBuffers),
    Image(WgpuImage),
}

struct WgpuVertexBuffers {
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
        bundle: &RenderBundle,
        renderer: &WgpuRenderer,
        renderer_targets: &RendererTargets,
        _dpi_scale_factor: f32,
    ) -> Self {
        let RenderBundle {
            world_set,
            screen_sets: bundle_screen_sets,
        } = bundle;
        let WorldRenderSet {
            poly_tessellation,
            points,
            images,
            clip_area,
            image_store,
            ..
        } = world_set;

        let clip_area_buffers = clip_area
            .as_ref()
            .map(|v| Self::write_poly_buffers(v, renderer));

        let poly_buffers = Self::write_poly_buffers(poly_tessellation, renderer);

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
            .map(|decoded_image| renderer.get_or_create_image_texture(decoded_image))
            .collect();

        let mut image_buffers = vec![];
        for image_info in images {
            let image = renderer_targets.pipelines.image_pipeline().create_image(
                &renderer.device,
                textures
                    .get(image_info.store_index)
                    .expect("texture at index must exist")
                    .clone(),
                &image_info.vertices,
            );
            image_buffers.push(image);
        }

        let mut screen_sets = vec![];
        for bundle_screen_set in bundle_screen_sets {
            let data = match &bundle_screen_set.data {
                ScreenSetData::Vertices(buffers) => {
                    let index_buffer =
                        renderer
                            .device
                            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: None,
                                contents: bytemuck::cast_slice(&buffers.indices),
                                usage: wgpu::BufferUsages::INDEX,
                            });

                    let vertex_buffer =
                        renderer
                            .device
                            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: None,
                                usage: wgpu::BufferUsages::VERTEX,
                                contents: bytemuck::cast_slice(&buffers.vertices),
                            });

                    let buffers = WgpuVertexBuffers {
                        index: index_buffer,
                        vertex: vertex_buffer,
                        index_count: buffers.indices.len() as u32,
                    };

                    WgpuScreenSetData::Vertex(buffers)
                }
                ScreenSetData::Image { vertices, bitmap } => {
                    let bind_group = renderer.get_or_create_image_texture(bitmap);
                    let image = renderer_targets
                        .pipelines
                        .screen_set_image_pipeline()
                        .create_image(&renderer.device, bind_group, vertices);
                    WgpuScreenSetData::Image(image)
                }
            };

            screen_sets.push(Arc::new(Mutex::new(WgpuScreenSet {
                state: RenderSetState::Hidden,
                animation_duration: bundle_screen_set.animation_duration,
                anchor_point: bundle_screen_set.anchor_point,
                bbox: bundle_screen_set.bbox,
                hide_on_overlay: bundle_screen_set.hide_on_overlay,
                data,
            })));
        }

        Self {
            clip_area_buffers,
            map_ref_buffers: poly_buffers,
            image_buffers,
            dot_buffers,
            screen_sets,
        }
    }

    fn write_poly_buffers(
        tessellation: &VertexBuffers<PolyVertex, u32>,
        renderer: &WgpuRenderer,
    ) -> WgpuVertexBuffers {
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

        WgpuVertexBuffers {
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

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct DisplayInstance {
    pub opacity: f32,
}

impl DisplayInstance {
    fn wgpu_desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<DisplayInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 10,
                format: wgpu::VertexFormat::Float32,
            }],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ScreenSetInstance {
    anchor: [f32; 3],
    opacity: f32,
}

impl ScreenSetInstance {
    fn wgpu_desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<ScreenSetInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
}
