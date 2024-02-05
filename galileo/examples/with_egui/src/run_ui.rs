use egui::Context;
use galileo_types::geo::impls::point::GeoPoint2d;
use galileo_types::geo::traits::point::GeoPoint;

#[derive(Clone, Default, Debug)]
pub struct UiState {
    pub positions: Positions,
}

#[derive(Clone, Default, Debug)]
pub struct Positions {
    pub pointer_position: Option<GeoPoint2d>,
    pub map_center_position: Option<GeoPoint2d>,
}

pub fn run_ui(state: &mut UiState, ui: &Context) {
    egui::Window::new("Galileo map").show(ui, |ui| {
        ui.label("Pointer position:");
        if let Some(pointer_position) = state.positions.pointer_position {
            ui.label(format!(
                "Lat: {:.4} Lon: {:.4}",
                pointer_position.lat(),
                pointer_position.lon()
            ));
        } else {
            ui.label("<unavaliable>");
        }

        ui.separator();

        ui.label("Map center position:");
        if let Some(map_center_position) = state.positions.map_center_position {
            ui.label(format!(
                "Lat: {:.4} Lon: {:.4}",
                map_center_position.lat(),
                map_center_position.lon()
            ));
        } else {
            ui.label("<unavaliable>");
        }
    });
}
