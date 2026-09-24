use crate::gui::drawables::DrawableManager;


pub struct DrawableSelectorPanel {
    
}

impl DrawableSelectorPanel {
    pub fn new() -> Self {
        Self {}
    }

    pub fn draw(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, unique_id: &str, drawable_manager: &mut DrawableManager) {
        egui::CollapsingHeader::new("Element Selector")
            .id_salt(format!("element-selector-{}", unique_id))
            .show(ui, |ui| {
                drawable_manager.draw_parameters(ui, unique_id);
            });
    }
}