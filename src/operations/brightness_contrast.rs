use eframe::egui;

use crate::operation::Operation;
use crate::widgets;

#[derive(Default)]
pub struct BrightnessContrast {
    brightness: f32,
    contrast: f32,
}

impl Operation for BrightnessContrast {
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
        for pixel in image.pixels_mut() {
            for channel in &mut pixel.0[..3] {
                let mut value = *channel as f32 / 255.0;
                value += self.brightness;
                value = (value - 0.5) * factor + 0.5;
                *channel = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
    }
}
