use eframe::egui;

pub trait Operation {
    fn name(&self) -> &'static str;

    fn apply(&self, image: &mut image::RgbaImage);

    fn has_settings(&self) -> bool {
        false
    }

    fn settings_ui(&mut self, _ui: &mut egui::Ui) -> bool {
        false
    }
}

pub struct OperationKind {
    pub menu_label: &'static str,
    pub make: fn() -> Box<dyn Operation>,
}

pub struct OperationGroup {
    pub label: &'static str,
    pub kinds: &'static [OperationKind],
}
