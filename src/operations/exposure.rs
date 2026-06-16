use eframe::egui;

use crate::operation::{par_pixels, Operation};
use crate::widgets;

pub struct Exposure {
    exposure: f32,
    offset: f32,
    gamma: f32,
}

impl Default for Exposure {
    fn default() -> Self {
        Self {
            exposure: 0.0,
            offset: 0.0,
            gamma: 1.0,
        }
    }
}

impl Operation for Exposure {
    fn name(&self) -> &'static str {
        "Exposure"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        changed |= widgets::slider(ui, "Exposure", &mut self.exposure, -3.0..=3.0);
        changed |= widgets::slider(ui, "Offset", &mut self.offset, -0.5..=0.5);
        changed |= widgets::slider(ui, "Gamma", &mut self.gamma, 0.1..=5.0);
        changed
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        let mult = 2f32.powf(self.exposure);
        let inv_gamma = 1.0 / self.gamma;
        par_pixels(image, |px| {
            for channel in &mut px[..3] {
                let mut value = *channel as f32 / 255.0 * mult + self.offset;
                value = value.max(0.0).powf(inv_gamma);
                *channel = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        });
    }
}
