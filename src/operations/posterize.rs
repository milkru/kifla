use eframe::egui;

use crate::operation::Operation;
use crate::widgets;

pub struct Posterize {
    levels: f32,
}

impl Default for Posterize {
    fn default() -> Self {
        Self { levels: 256.0 }
    }
}

impl Operation for Posterize {
    fn name(&self) -> &'static str {
        "Posterize"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        widgets::slider(ui, "Levels", &mut self.levels, 2.0..=256.0)
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        let steps = (self.levels.round() - 1.0).max(1.0);
        for pixel in image.pixels_mut() {
            for channel in &mut pixel.0[..3] {
                let value = *channel as f32 / 255.0;
                let quantized = (value * steps).round() / steps;
                *channel = (quantized.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
    }
}
