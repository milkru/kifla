use eframe::egui;

use crate::modifier::Modifier;
use crate::pixel::map_rgb;
use crate::widgets;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct BrightnessContrast {
    brightness: f32,
    contrast: f32,
}

impl Modifier for BrightnessContrast {
    crate::modifier_serde!("brightness_contrast");

    fn name(&self) -> &'static str {
        "Brightness / Contrast"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        changed |= widgets::slider(ui, "Brightness", &mut self.brightness, -1.0..=1.0);
        changed |= widgets::slider(ui, "Contrast", &mut self.contrast, -1.0..=1.0);
        changed
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        let factor = 1.0 + self.contrast;
        map_rgb(image, |value| {
            (value + self.brightness - 0.5) * factor + 0.5
        });
    }
}
