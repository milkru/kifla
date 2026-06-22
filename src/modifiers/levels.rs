use eframe::egui;

use crate::modifier::Modifier;
use crate::pixel::map_rgb;
use crate::widgets;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Levels {
    in_black: f32,
    in_white: f32,
    gamma: f32,
    out_black: f32,
    out_white: f32,
}

impl Default for Levels {
    fn default() -> Self {
        Self {
            in_black: 0.0,
            in_white: 1.0,
            gamma: 1.0,
            out_black: 0.0,
            out_white: 1.0,
        }
    }
}

impl Modifier for Levels {
    crate::modifier_serde!("levels");

    fn name(&self) -> &'static str {
        "Levels"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        changed |= widgets::slider(ui, "Input black", &mut self.in_black, 0.0..=1.0);
        changed |= widgets::slider(ui, "Input white", &mut self.in_white, 0.0..=1.0);
        changed |= widgets::slider(ui, "Gamma", &mut self.gamma, 0.1..=5.0);
        ui.separator();
        changed |= widgets::slider(ui, "Output black", &mut self.out_black, 0.0..=1.0);
        changed |= widgets::slider(ui, "Output white", &mut self.out_white, 0.0..=1.0);
        changed
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        let denom = (self.in_white - self.in_black).max(1e-4);
        let inv_gamma = 1.0 / self.gamma;
        map_rgb(image, |value| {
            let value = ((value - self.in_black) / denom)
                .clamp(0.0, 1.0)
                .powf(inv_gamma);
            self.out_black + value * (self.out_white - self.out_black)
        });
    }
}
