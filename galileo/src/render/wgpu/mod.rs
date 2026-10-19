use std::any::Any;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::mem::size_of;
use std::sync::{Arc, Weak};
use std::time::Duration;

use ahash::HashMap;
use cfg_if::cfg_if;
use effects::horizon::HorizonPipeline;
use galileo_types::cartesian::{Rect, Size, Vector2};
use lyon::tessellation::VertexBuffers;
use nalgebra::{Point4, Rotation3, Vector3};
use parking_lot::Mutex;
use wgpu::util::DeviceExt;
use wgpu::{
    Adapter, BindGroup, Buffer, BufferAddress, BufferDescriptor, BufferUsages,
    COPY_BYTES_PER_ROW_ALIGNMENT, CommandEncoder, CurrentSurfaceTexture, Device, Extent3d,
    Origin3d, Queue, RenderPassDepthStencilAttachment, StoreOp, Surface, SurfaceConfiguration,
    SurfaceStatus, SurfaceTexture, TexelCopyBufferInfo, TexelCopyBufferLayout,
    TexelCopyTextureInfo, Texture, TextureAspect, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TextureView, TextureViewDescriptor, WasmNotSendSync,
};

use super::render_bundle::screen_set::{RenderSetState, ScreenSetData};
use super::{BundleToDraw, Canvas, PackedBundle, RenderOptions};
use crate::Color;
use crate::decoded_image::DecodedImage;
use crate::error::GalileoError;
use crate::map::Map;
use crate::render::render_bundle::RenderBundle;
use crate::render::render_bundle::world_set::{PointInstance, PolyVertex, WorldRenderSet};
use crate::render::wgpu::pipelines::Pipelines;
use crate::render::wgpu::pipelines::image::WgpuImage;
use crate::view::MapView;

mod effects;
mod pipelines;

pub use effects::horizon::HorizonOptions;

const DEFAULT_BACKGROUND: Color = Color::WHITE;
const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth24PlusStencil8;
const TARGET_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

type TexturesMap = HashMap<u64, (Weak<DecodedImage>, Arc<BindGroup>)>;

/// Render backend that uses `wgpu` crate to render the map.
pub struct WgpuRenderer {
    device: Device,
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

    fn present(self, queue: &Queue) {
        match self {
            RenderTargetTexture::Surface(t) => queue.present(t),
            RenderTargetTexture::Texture(_) => {}
        }
    }
}

impl RenderTarget {
    fn texture(&self) -> Result<RenderTargetTexture<'_>, SurfaceStatus> {
        match &self {
            RenderTarget::Surface { surface, .. } => {
                let texture = match surface.get_current_texture() {
                    CurrentSurfaceTexture::Success(texture) => texture,
                    CurrentSurfaceTexture::Suboptimal(texture) => texture,
                    CurrentSurfaceTexture::Lost => return Err(SurfaceStatus::Lost),
                    CurrentSurfaceTexture::Occluded => return Err(SurfaceStatus::Occluded),
                    CurrentSurfaceTexture::Outdated => return Err(SurfaceStatus::Outdated),
                    CurrentSurfaceTexture::Timeout => return Err(SurfaceStatus::Timeout),
                    CurrentSurfaceTexture::Validation => return Err(SurfaceStatus::Validation),
                };
                Ok(RenderTargetTexture::Surface(texture))
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
    /// The renderer owns no queue, so the returned [`Queue`] must be passed
    /// to the submitting render methods. Returns `None` if a device adapter
    /// cannot be acquired.
    pub async fn new() -> Option<(Self, Queue)> {
        let instance = Self::create_instance();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .ok()?;

        let (device, queue) = Self::create_device(&adapter).await;

        Some((
            Self {
                device,
                renderer_targets: None,
                background: DEFAULT_BACKGROUND,
                textures: Default::default(),
                horizon_options: Some(HorizonOptions::default()),
            },
            queue,
        ))
    }

    /// Creates a new wgpu renderer that renders the map to an image buffer of the given size.
    ///
    /// Returns `None` if a device adapter cannot be acquired.
    pub async fn new_with_texture_rt(size: Size<u32>) -> Option<(Self, Queue)> {
        let (mut renderer, queue) = Self::new().await?;
        renderer.init_target_texture(size);

        Some((renderer, queue))
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
    pub async fn new_with_window<W>(window: Arc<W>, size: Size<u32>) -> Option<(Self, Queue)>
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

        let renderer = Self::new_with_device_and_surface(device, surface, config);

        Some((renderer, queue))
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
                apply_limit_buckets: false,
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

        SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: TARGET_TEXTURE_FORMAT,
            width: size.width(),
            height: size.height(),
            present_mode: surface_caps.present_modes[0],
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            color_space: wgpu::SurfaceColorSpace::Auto,
            view_formats: vec![],
        }
    }

    /// Creates a new renderer from the initialized wgpu structs.
    ///
    /// The renderer stores no queue, so pass one to the submitting render
    /// methods.
    pub fn new_with_device_and_surface(
        device: Device,
        surface: Surface<'static>,
        config: SurfaceConfiguration,
    ) -> Self {
        let render_target = RenderTarget::Surface { surface, config };
        let mut renderer = Self {
            device,
            renderer_targets: None,
            background: DEFAULT_BACKGROUND,
            textures: Default::default(),
            horizon_options: Some(HorizonOptions::default()),
        };
        renderer.init_renderer_targets(render_target);

        renderer
    }

    /// Creates a new rendering using the given wgpu device, rendering to a texture
    /// with the given size.
    pub fn new_with_device_and_texture(device: Device, size: Size<u32>) -> Self {
        let mut renderer = Self {
            device,
            renderer_targets: None,
            background: DEFAULT_BACKGROUND,
            textures: Default::default(),
            horizon_options: Some(HorizonOptions::default()),
        };

        renderer.init_target_texture(size);

        renderer
    }

    /// Creates a renderer that records drawing into externally supplied wgpu
    /// command encoders instead of submitting work itself.
    ///
    /// The renderer never owns a queue. Use
    /// [`Self::render_to_texture_view_with_encoder`] to record into a
    /// caller-owned encoder, or pass a queue explicitly to the submitting
    /// methods ([`Self::render`], [`Self::render_to_texture_view`] and
    /// [`Self::get_image`]). `device` must own the target texture views the
    /// renderer draws into.
    pub fn new_with_device(device: Device, size: Size<u32>) -> Self {
        let mut renderer = Self {
            device,
            renderer_targets: None,
            background: DEFAULT_BACKGROUND,
            textures: Default::default(),
            horizon_options: None,
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

        wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
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
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
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
    ///
    /// The recorded readback commands are submitted on the given queue.
    pub async fn get_image(&self, queue: &Queue) -> Result<Vec<u8>, SurfaceStatus> {
        let Some(renderer_targets) = &self.renderer_targets else {
            return Err(SurfaceStatus::Lost);
        };

        let render_target_size = renderer_targets.render_target.size();

        const RGBA_BYTES_PER_PIXEL: u32 = 4;
        let bytes_per_row = render_target_size.width() * RGBA_BYTES_PER_PIXEL;
        let bytes_per_row_aligned = match bytes_per_row / COPY_BYTES_PER_ROW_ALIGNMENT {
            0 => bytes_per_row,
            v => (v + 1) * COPY_BYTES_PER_ROW_ALIGNMENT,
        };

        let buffer_size = (bytes_per_row_aligned * render_target_size.height()) as BufferAddress;
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
                    bytes_per_row: Some(bytes_per_row_aligned),
                    rows_per_image: Some(render_target_size.height()),
                },
            },
            Extent3d {
                width: render_target_size.width(),
                height: render_target_size.height(),
                depth_or_array_layers: 1,
            },
        );

        queue.submit(Some(encoder.finish()));

        let buffer_slice = buffer.slice(..);
        let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            if let Err(err) = tx.send(result) {
                log::error!("Failed to send by channel: {err:?}");
            }
        });

        if let Err(err) = self.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(60)),
        }) {
            log::error!("polling device failed: {err:?}");
        }

        match rx.receive().await {
            Some(result) => match result {
                Ok(()) => {}
                Err(err) => {
                    log::error!("Writing to image buffer failed: {err:?}.");
                    return Err(SurfaceStatus::Lost);
                }
            },
            None => {
                log::error!("Channel was closed");
                return Err(SurfaceStatus::Lost);
            }
        }

        let data = buffer_slice
            .get_mapped_range()
            .expect("failed to map the readback buffer");
        let mut pixels = vec![
            0u8;
            (render_target_size.width() * render_target_size.height() * RGBA_BYTES_PER_PIXEL)
                as usize
        ];

        for row_index in 0..render_target_size.height() {
            let first_byte_index_buf = (row_index * bytes_per_row_aligned) as usize;
            let last_byte_index_buf = first_byte_index_buf + bytes_per_row as usize;

            let first_byte_index_pix = (row_index * bytes_per_row) as usize;
            let last_byte_index_pix = first_byte_index_pix + bytes_per_row as usize;

            pixels[first_byte_index_pix..last_byte_index_pix]
                .copy_from_slice(&data[first_byte_index_buf..last_byte_index_buf]);
        }

        Ok(pixels)
    }

    /// Renders the map to the given texture and submits the work on the given
    /// queue.
    pub fn render_to_texture_view(&self, map: &Map, view: &TextureView, queue: &Queue) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.render_to_texture_view_with_encoder(map, view, &mut encoder);

        queue.submit(Some(encoder.finish()));

        if let Some(targets) = &self.renderer_targets {
            self.draw_horizon(map.view(), targets, view, queue);
        }
    }

    /// Records the map render into `encoder`, resolving into `view`.
    ///
    /// Nothing is submitted by this method: the caller owns the encoder and
    /// decides when and where its commands are submitted. Every pipeline,
    /// buffer and texture used here is created on the same device that owns
    /// `view`, so this supports rendering into externally managed textures.
    /// The horizon effect is not recorded by this method; it is applied by
    /// the legacy [`Self::render_to_texture_view`] path instead.
    pub fn render_to_texture_view_with_encoder(
        &self,
        map: &Map,
        view: &TextureView,
        encoder: &mut CommandEncoder,
    ) {
        let Some(renderer_targets) = &self.renderer_targets else {
            return;
        };

        {
            let background = self.background.to_f32_array();
            let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &renderer_targets.multisampling_view,
                    depth_slice: None,
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
                multiview_mask: None,
            });
        }

        self.render_map(map, view, encoder, None);
    }

    /// Renders the map and submits the recorded work on the given queue.
    pub fn render(&self, map: &Map, queue: &Queue) -> Result<(), SurfaceStatus> {
        let Some(renderer_targets) = &self.renderer_targets else {
            return Ok(());
        };

        self.trim_textures();

        let texture = renderer_targets.render_target.texture()?;
        let view = texture.view();

        self.render_to_texture_view(map, &view, queue);

        texture.present(queue);

        Ok(())
    }

    fn render_map(
        &self,
        map: &Map,
        texture_view: &TextureView,
        encoder: &mut CommandEncoder,
        queue: Option<&Queue>,
    ) {
        let view = map.view();
        let Some(renderer_targets) = &self.renderer_targets else {
            return;
        };

        let Some(mut canvas) = WgpuCanvas::new(
            self,
            renderer_targets,
            texture_view,
            view.clone(),
            encoder,
            queue,
        ) else {
            log::warn!("Layer cannot be rendered to the map view.");
            return;
        };

        for layer in map.layers().iter_visible() {
            layer.render(view, &mut canvas);
        }

        let needs_animation = canvas.draw_screen_sets();
        canvas.flush_pending_uploads();
        if needs_animation {
            map.redraw();
        }
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
        queue: &Queue,
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
                    depth_slice: None,
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
                multiview_mask: None,
            });

            renderer_targets.pipelines.set_bindings(&mut render_pass);
            pipeline.render(view, queue, &mut render_pass, 0);
        }

        queue.submit(std::iter::once(encoder.finish()));
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

    fn get_or_create_image_texture(
        &self,
        image: &Arc<DecodedImage>,
        pending: &Mutex<Vec<ImageUpload>>,
        queue: Option<&Queue>,
    ) -> Arc<BindGroup> {
        let mut hasher = ahash::AHasher::default();
        image.hash(&mut hasher);
        let hash = hasher.finish();

        if let Some((_, texture)) = self.textures.lock().get(&hash) {
            return texture.clone();
        }

        let created = self
            .renderer_targets
            .as_ref()
            .expect("trying to use pipelines of uninitialized renderer")
            .pipelines
            .create_image_texture(&self.device, queue, image);

        if let Some(staging) = created.staging {
            pending.lock().push(ImageUpload {
                texture: created.texture,
                staging,
                width: created.width,
                height: created.height,
                bytes_per_row: created.bytes_per_row,
            });
        }

        self.textures
            .lock()
            .insert(hash, (Arc::downgrade(image), created.bind_group.clone()));

        created.bind_group
    }
}

/// A newly created image texture together with the staging buffer that carries
/// its pixels. The staging buffer is uploaded with an encoder copy before the
/// first render pass that samples the texture.
struct ImageUpload {
    texture: Texture,
    staging: Buffer,
    width: u32,
    height: u32,
    bytes_per_row: u32,
}

struct WgpuCanvas<'a> {
    renderer: &'a WgpuRenderer,
    renderer_targets: &'a RendererTargets,
    view: &'a TextureView,
    encoder: &'a mut CommandEncoder,
    queue: Option<&'a Queue>,
    map_view: MapView,

    pending_uploads: Mutex<Vec<ImageUpload>>,
    screen_sets: Vec<(Arc<Mutex<WgpuScreenSet>>, f32, Vector2<f32>)>,
}

impl<'a> WgpuCanvas<'a> {
    fn new(
        renderer: &'a WgpuRenderer,
        renderer_targets: &'a RendererTargets,
        view: &'a TextureView,
        map_view: MapView,
        encoder: &'a mut CommandEncoder,
        queue: Option<&'a Queue>,
    ) -> Option<Self> {
        let rotation_mtx = Rotation3::new(Vector3::new(
            map_view.rotation_x(),
            0.0,
            -map_view.rotation_z(),
        ))
        .to_homogeneous();
        let staging = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Map view staging buffer"),
                usage: wgpu::BufferUsages::COPY_SRC,
                contents: bytemuck::cast_slice(&[ViewUniform {
                    view_proj: map_view.map_to_scene_mtx()?,
                    view_rotation: rotation_mtx.cast::<f32>().data.0,
                    inv_screen_size: [
                        1.0 / renderer.size().width() as f32,
                        1.0 / renderer.size().height() as f32,
                    ],
                    resolution: map_view.resolution() as f32,
                    _padding: [0.0; 1],
                }]),
            });
        encoder.copy_buffer_to_buffer(
            &staging,
            0,
            renderer_targets.pipelines.map_view_buffer(),
            0,
            size_of::<ViewUniform>() as u64,
        );

        Some(Self {
            renderer,
            renderer_targets,
            view,
            encoder,
            queue,
            map_view,
            pending_uploads: Mutex::new(Vec::new()),
            screen_sets: vec![],
        })
    }

    fn flush_pending_uploads(&mut self) {
        let pending = std::mem::take(&mut *self.pending_uploads.lock());
        for upload in pending {
            self.encoder.copy_buffer_to_texture(
                TexelCopyBufferInfo {
                    buffer: &upload.staging,
                    layout: TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(upload.bytes_per_row),
                        rows_per_image: Some(upload.height),
                    },
                },
                TexelCopyTextureInfo {
                    aspect: TextureAspect::All,
                    texture: &upload.texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                },
                Extent3d {
                    width: upload.width,
                    height: upload.height,
                    depth_or_array_layers: 1,
                },
            );
        }
    }
}

impl Canvas for WgpuCanvas<'_> {
    fn size(&self) -> Size {
        self.renderer.size()
    }

    fn pack_bundle(&self, bundle: &RenderBundle) -> Box<dyn PackedBundle> {
        Box::new(WgpuPackedBundle::new(
            bundle,
            self.renderer,
            self.renderer_targets,
            &self.pending_uploads,
            self.queue,
        ))
    }

    fn draw_bundles(&mut self, bundles: &[super::BundleToDraw], options: RenderOptions) {
        if bundles.is_empty() {
            log::debug!("Requested drawing of 0 bundles");
            return;
        }

        self.flush_pending_uploads();
        let encoder = &mut *self.encoder;

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
                    depth_slice: None,
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
                multiview_mask: None,
            });

            let display_instances: Vec<_> = bundles
                .iter()
                .map(
                    |BundleToDraw {
                         opacity, offset, ..
                     }| DisplayInstance {
                        opacity: *opacity,
                        offset: [offset.dx(), offset.dy(), 0.0],
                    },
                )
                .collect();

            let display_buffer =
                self.renderer
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        usage: wgpu::BufferUsages::VERTEX,
                        contents: bytemuck::cast_slice(&display_instances),
                    });
            render_pass.set_vertex_buffer(1, display_buffer.slice(..));

            for (
                index,
                BundleToDraw {
                    bundle,
                    opacity,
                    offset,
                },
            ) in bundles.iter().enumerate()
            {
                if let Some(cast) = bundle.as_any().downcast_ref::<WgpuPackedBundle>() {
                    self.renderer_targets.pipelines.render(
                        &mut render_pass,
                        cast,
                        options,
                        index as u32,
                    );

                    for screen_set in &cast.screen_sets {
                        self.screen_sets
                            .push((screen_set.clone(), *opacity, *offset));
                    }
                }
            }
        }
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

        self.flush_pending_uploads();
        let screen_sets = std::mem::take(&mut self.screen_sets);
        let mut sets: Vec<_> = screen_sets
            .iter()
            .filter_map(|(set, _, offset)| {
                let Some(locked) = set.try_lock() else {
                    // TODO: this means that the same tile is reused. We just wait for it to
                    // disappear from the map. This would result in some visual bugs, but not so
                    // critical as to be blocked by it ATM.
                    return None;
                };

                let projected_anchor = transform
                    * Point4::new(
                        locked.anchor_point[0] as f64 + offset.dx() as f64,
                        locked.anchor_point[1] as f64 + offset.dy() as f64,
                        locked.anchor_point[2] as f64,
                        1.0,
                    );
                let normalaized = projected_anchor / projected_anchor.w.abs();

                Some((locked, normalaized, offset))
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
            .filter_map(|(mut set, anchor, offset)| {
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

                            Some((set, offset))
                        }
                        RenderSetState::Displayed => {
                            set.state = RenderSetState::FadingOut {
                                start_time: web_time::Instant::now(),
                            };
                            Some((set, offset))
                        }
                        RenderSetState::FadingOut { .. } => Some((set, offset)),
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

                    Some((set, offset))
                }
            })
            .collect();

        let mut is_animating = false;
        let encoder = &mut *self.encoder;

        {
            let view = &self.renderer_targets.multisampling_view;
            let resolve_target = Some(self.view);
            let depth_view = &self.renderer_targets.stencil_view_multisample;

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
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
                multiview_mask: None,
            });

            let instances: Vec<DisplayInstance> = filtered_sets
                .iter_mut()
                .map(|(set, offset)| {
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

                    let anchor = [
                        set.anchor_point[0] + offset.dx(),
                        set.anchor_point[1] + offset.dy(),
                        set.anchor_point[2],
                    ];

                    DisplayInstance {
                        opacity,
                        offset: anchor,
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
                    &set.0.data,
                    &mut render_pass,
                    index as u32,
                );
            }
        }

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
        pending: &Mutex<Vec<ImageUpload>>,
        queue: Option<&Queue>,
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
            .map(|decoded_image| {
                renderer.get_or_create_image_texture(decoded_image, pending, queue)
            })
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
                    let bind_group = renderer.get_or_create_image_texture(bitmap, pending, queue);
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
    pub offset: [f32; 3],
}

impl DisplayInstance {
    fn wgpu_desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<DisplayInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<f32>() as wgpu::BufferAddress,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}
