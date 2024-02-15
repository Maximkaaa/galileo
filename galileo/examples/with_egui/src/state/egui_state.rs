use egui::Context;
use egui_wgpu::Renderer;
use egui_wgpu::ScreenDescriptor;

use egui_winit::{EventResponse, State};
use wgpu::{Device, TextureFormat};
use winit::event::WindowEvent;
use winit::window::Window;

use super::WgpuFrame;

pub struct EguiState {
    context: Context,
    state: State,
    renderer: Renderer,
}

impl EguiState {
    pub fn new(
        device: &Device,
        output_color_format: TextureFormat,
        output_depth_format: Option<TextureFormat>,
        msaa_samples: u32,
        window: &Window,
    ) -> EguiState {
        let egui_context = Context::default();
        let id = egui_context.viewport_id();

        let visuals = Default::default();
        egui_context.set_visuals(visuals);
        egui_context.set_pixels_per_point(window.scale_factor() as f32);

        let egui_state = State::new(egui_context.clone(), id, &window, None, None);

        let egui_renderer = Renderer::new(
            device,
            output_color_format,
            output_depth_format,
            msaa_samples,
        );

        EguiState {
            context: egui_context,
            state: egui_state,
            renderer: egui_renderer,
        }
    }

    pub fn handle_event(&mut self, window: &Window, event: &WindowEvent) -> EventResponse {
        let mut response = self.state.on_window_event(window, event);
        if self.context.wants_pointer_input() {
            response.consumed = true;
        }

        response
    }

    pub fn render(&mut self, wgpu_frame: &mut WgpuFrame<'_>, run_ui: impl FnOnce(&Context)) {
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [wgpu_frame.size.width, wgpu_frame.size.height],
            pixels_per_point: wgpu_frame.window.scale_factor() as f32,
        };

        self.context
            .set_pixels_per_point(wgpu_frame.window.scale_factor() as f32);

        let raw_input = self.state.take_egui_input(wgpu_frame.window);
        let full_output = self.context.run(raw_input, run_ui);

        self.state
            .handle_platform_output(wgpu_frame.window, full_output.platform_output);

        let paint_jobs = self
            .context
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(wgpu_frame.device, wgpu_frame.queue, *id, image_delta);
        }

        self.renderer.update_buffers(
            wgpu_frame.device,
            wgpu_frame.queue,
            wgpu_frame.encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        {
            let mut render_pass =
                wgpu_frame
                    .encoder
                    .begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: wgpu_frame.texture_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        label: Some("egui render pass"),
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

            self.renderer
                .render(&mut render_pass, &paint_jobs, &screen_descriptor);
        }

        for x in &full_output.textures_delta.free {
            self.renderer.free_texture(x)
        }
    }
}
