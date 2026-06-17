use eframe::egui;

use crate::edit::Edit;
use crate::pixel::map_rgb;
use crate::widgets;

#[derive(serde::Serialize, serde::Deserialize)]
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

impl Edit for Exposure {
    crate::edit_serde!("exposure");

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
        map_rgb(image, |value| {
            (value * mult + self.offset).max(0.0).powf(inv_gamma)
        });
    }
}
