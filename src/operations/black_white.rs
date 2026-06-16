use eframe::egui;

use crate::operation::Operation;
use crate::widgets;

pub struct BlackWhite {
    red: f32,
    green: f32,
    blue: f32,
}

impl Default for BlackWhite {
    fn default() -> Self {
        Self {
            red: 0.3,
            green: 0.59,
            blue: 0.11,
        }
    }
}

impl Operation for BlackWhite {
    fn name(&self) -> &'static str {
        "Black & White"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        changed |= widgets::slider(ui, "Red", &mut self.red, -0.5..=1.5);
        changed |= widgets::slider(ui, "Green", &mut self.green, -0.5..=1.5);
        changed |= widgets::slider(ui, "Blue", &mut self.blue, -0.5..=1.5);
        changed
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        for pixel in image.pixels_mut() {
            let gray = (pixel[0] as f32 / 255.0 * self.red
                + pixel[1] as f32 / 255.0 * self.green
                + pixel[2] as f32 / 255.0 * self.blue)
                .clamp(0.0, 1.0);
            let value = (gray * 255.0).round() as u8;
            pixel[0] = value;
            pixel[1] = value;
            pixel[2] = value;
        }
    }
}
