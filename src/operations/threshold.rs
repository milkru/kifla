use eframe::egui;

use crate::operation::Operation;
use crate::widgets;

pub struct Threshold {
    level: f32,
}

impl Default for Threshold {
    fn default() -> Self {
        Self { level: 0.5 }
    }
}

impl Operation for Threshold {
    fn name(&self) -> &'static str {
        "Threshold"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        widgets::slider(ui, "Threshold", &mut self.level, 0.0..=1.0)
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        for pixel in image.pixels_mut() {
            let lum = (0.299 * pixel[0] as f32 + 0.587 * pixel[1] as f32 + 0.114 * pixel[2] as f32)
                / 255.0;
            let value = if lum >= self.level { 255 } else { 0 };
            pixel[0] = value;
            pixel[1] = value;
            pixel[2] = value;
        }
    }
}
