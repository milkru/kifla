use eframe::egui;

use crate::operation::{par_pixels, Operation};
use crate::widgets;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Threshold {
    level: f32,
    amount: f32,
}

impl Default for Threshold {
    fn default() -> Self {
        Self {
            level: 0.5,
            amount: 0.0,
        }
    }
}

impl Operation for Threshold {
    crate::op_serde!("threshold");

    fn name(&self) -> &'static str {
        "Threshold"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        changed |= widgets::slider(ui, "Threshold", &mut self.level, 0.0..=1.0);
        changed |= widgets::slider(ui, "Amount", &mut self.amount, 0.0..=1.0);
        changed
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        if self.amount <= 0.0 {
            return;
        }
        par_pixels(image, |px| {
            let lum = (0.299 * px[0] as f32 + 0.587 * px[1] as f32 + 0.114 * px[2] as f32) / 255.0;
            let value = if lum >= self.level { 255.0 } else { 0.0 };
            for channel in &mut px[..3] {
                let blended = *channel as f32 * (1.0 - self.amount) + value * self.amount;
                *channel = blended.round() as u8;
            }
        });
    }
}
