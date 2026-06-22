use eframe::egui;

use crate::modifier::Modifier;
use crate::pixel::par_pixels;
use crate::widgets;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BlackWhite {
    red: f32,
    green: f32,
    blue: f32,
    amount: f32,
}

impl Default for BlackWhite {
    fn default() -> Self {
        Self {
            red: 0.3,
            green: 0.59,
            blue: 0.11,
            amount: 0.0,
        }
    }
}

impl Modifier for BlackWhite {
    crate::modifier_serde!("black_white");

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
        ui.separator();
        changed |= widgets::slider(ui, "Amount", &mut self.amount, 0.0..=1.0);
        changed
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        if self.amount <= 0.0 {
            return;
        }
        par_pixels(image, |px| {
            let gray = (px[0] as f32 / 255.0 * self.red
                + px[1] as f32 / 255.0 * self.green
                + px[2] as f32 / 255.0 * self.blue)
                .clamp(0.0, 1.0)
                * 255.0;
            for channel in &mut px[..3] {
                let blended = *channel as f32 * (1.0 - self.amount) + gray * self.amount;
                *channel = blended.round() as u8;
            }
        });
    }
}
