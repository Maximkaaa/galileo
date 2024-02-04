use egui::Context;

#[derive(Clone, Default, Debug)]
pub struct UiState {
    pub name: String,
    pub age: u32,
}

pub fn run_ui(state: &mut UiState, ui: &Context) {
    egui::Window::new("My egui Application").show(ui, |ui| {
        ui.horizontal(|ui| {
            let name_label = ui.label("Your name: ");
            ui.text_edit_singleline(&mut state.name)
                .labelled_by(name_label.id);
        });
        ui.add(egui::Slider::new(&mut state.age, 0..=120).text("age"));
        if ui.button("Increment").clicked() {
            state.age += 1;
        }
        ui.label(format!("Hello '{}', age {}", state.name, state.age));
    });
}
