use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use egui::load::SizedTexture;
use egui::{Event, Image, ImageSource, Sense, TextureId, Ui, Vec2};
use egui_wgpu::wgpu::{FilterMode, TextureView};
use egui_wgpu::RenderState;
use galileo::control::{
    EventProcessor, MapController, MouseButton, RawUserEvent, UserEventHandler,
};
use galileo::galileo_types::cartesian::{Point2d, Size};
use galileo::galileo_types::geo::impls::GeoPoint2d;
use galileo::render::WgpuRenderer;
use galileo::{Map, Messenger};

pub struct EguiMap<'a> {
    state: &'a mut EguiMapState,
    position: Option<&'a mut GeoPoint2d>,
    resolution: Option<&'a mut f64>,
}

impl<'a> EguiMap<'a> {
    pub fn new(state: &'a mut EguiMapState) -> Self {
        Self {
            state,
            position: None,
            resolution: None,
        }
    }

    pub fn with_position(&'a mut self, position: &'a mut GeoPoint2d) -> &'a mut Self {
        let curr_view = self.state.map.view();
        if curr_view.position() != Some(*position) {
            self.state.map.set_view(curr_view.with_position(position));
        }

        self.position = Some(position);
        self
    }

    pub fn with_resolution(&'a mut self, resolution: &'a mut f64) -> &'a mut Self {
        let curr_view = self.state.map.view();
        if curr_view.resolution() != *resolution {
            self.state
                .map
                .set_view(curr_view.with_resolution(*resolution));
        }

        self.resolution = Some(resolution);
        self
    }

    pub fn show_ui(&mut self, ui: &mut Ui) {
        self.state.render(ui);

        let updated_view = self.state.map.view();
        if let Some(resolution) = &mut self.resolution {
            **resolution = updated_view.resolution();
        }

        if let Some(position) = &mut self.position {
            if let Some(view_position) = updated_view.position() {
                **position = view_position;
            }
        }
    }
}

pub struct EguiMapState {
    map: Map,
    egui_render_state: RenderState,
    renderer: WgpuRenderer,
    requires_redraw: Arc<AtomicBool>,
    texture_id: TextureId,
    texture_view: TextureView,
    event_processor: EventProcessor,
}

impl EguiMapState {
    pub fn new(
        mut map: Map,
        ctx: egui::Context,
        render_state: RenderState,
        handlers: impl IntoIterator<Item = Box<dyn UserEventHandler>>,
    ) -> Self {
        let requires_redraw = Arc::new(AtomicBool::new(true));
        let messenger = MapStateMessenger {
            context: ctx.clone(),
            requires_redraw: requires_redraw.clone(),
        };

        map.set_messenger(Some(messenger.clone()));
        for layer in map.layers_mut().iter_mut() {
            layer.set_messenger(Box::new(messenger.clone()));
        }

        // Set a default size so that render target can be created.
        // This size will be replaced by the UI on the first frame.
        let size = Size::new(1, 1);
        map.set_size(size.cast());

        let renderer = WgpuRenderer::new_with_device_and_texture(
            render_state.device.clone(),
            render_state.queue.clone(),
            size,
        );
        let texture = renderer
            .get_target_texture_view()
            .expect("failed to get map texture");
        let texture_id = render_state.renderer.write().register_native_texture(
            &render_state.device,
            &texture,
            FilterMode::Nearest,
        );

        let mut event_processor = EventProcessor::default();
        for handler in handlers {
            event_processor.add_handler_boxed(handler);
        }
        event_processor.add_handler(MapController::default());

        Self {
            map,
            egui_render_state: render_state,
            renderer,
            requires_redraw,
            texture_id,
            texture_view: texture,
            event_processor,
        }
    }

    pub fn request_redraw(&self) {
        self.map.redraw();
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        let available_size = ui.available_size();
        let map_size = self.renderer.size().cast::<f32>();

        let (rect, response) = ui.allocate_exact_size(available_size, Sense::click_and_drag());

        if self.event_processor.is_dragging() || response.contains_pointer() {
            let events = ui.input(|input_state| input_state.events.clone());
            self.process_events(&events);
        }

        self.map.animate();

        if available_size[0] != map_size.width() || available_size[1] != map_size.height() {
            self.resize_map(available_size);
        }

        if self.requires_redraw.swap(false, Ordering::Relaxed) {
            self.draw();
        }

        Image::new(ImageSource::Texture(SizedTexture::new(
            self.texture_id,
            Vec2::new(map_size.width(), map_size.height()),
        )))
        .paint_at(ui, rect);
    }

    fn resize_map(&mut self, size: Vec2) {
        log::trace!("Resizing map to size: {size:?}");

        let size = Size::new(size.x as f64, size.y as f64);
        self.map.set_size(size);

        let size = Size::new(size.width() as u32, size.height() as u32);
        self.renderer.resize(size);

        // After renderer is resized, a new texture is created, so we need to update its id that we
        // use in UI.
        let texture = self
            .renderer
            .get_target_texture_view()
            .expect("failed to get map texture");
        let texture_id = self
            .egui_render_state
            .renderer
            .write()
            .register_native_texture(
                &self.egui_render_state.device,
                &texture,
                FilterMode::Nearest,
            );

        self.texture_id = texture_id;
        self.texture_view = texture;

        self.map.redraw();
    }

    fn draw(&mut self) {
        log::trace!("Redrawing the map");
        self.map.load_layers();
        self.renderer
            .render_to_texture_view(&self.map, &self.texture_view);
    }

    fn process_events(&mut self, events: &[Event]) {
        for event in events {
            if let Some(raw_event) = Self::convert_event(event) {
                self.event_processor.handle(raw_event, &mut self.map);
            }
        }
    }

    fn convert_event(event: &Event) -> Option<RawUserEvent> {
        match event {
            Event::PointerButton {
                button, pressed, ..
            } => {
                let button = match button {
                    egui::PointerButton::Primary => MouseButton::Left,
                    egui::PointerButton::Secondary => MouseButton::Right,
                    egui::PointerButton::Middle => MouseButton::Middle,
                    _ => MouseButton::Other,
                };

                Some(match pressed {
                    true => RawUserEvent::ButtonPressed(button),
                    false => RawUserEvent::ButtonReleased(button),
                })
            }
            Event::PointerMoved(position) => {
                let scale = 1.0;
                let pointer_position =
                    Point2d::new(position.x as f64 / scale, position.y as f64 / scale);
                Some(RawUserEvent::PointerMoved(pointer_position))
            }
            Event::MouseWheel { delta, .. } => {
                let zoom = delta[1] as f64;
                if zoom.abs() < 0.0001 {
                    return None;
                }

                Some(RawUserEvent::Scroll(zoom))
            }

            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MapStateMessenger {
    pub requires_redraw: Arc<AtomicBool>,
    pub context: egui::Context,
}

impl Messenger for MapStateMessenger {
    fn request_redraw(&self) {
        log::trace!("Redraw requested");
        if !self.requires_redraw.swap(true, Ordering::Relaxed) {
            self.context.request_repaint();
        }
    }
}
