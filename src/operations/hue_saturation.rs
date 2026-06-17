use eframe::egui;

use crate::color;
use crate::operation::{par_pixels, Operation};
use crate::widgets;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct HueSaturation {
    hue: f32,
    saturation: f32,
    lightness: f32,
}

impl Operation for HueSaturation {
    crate::op_serde!("hue_saturation");

    fn name(&self) -> &'static str {
        "Hue / Saturation"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        changed |= widgets::slider(ui, "Hue", &mut self.hue, -180.0..=180.0);
        changed |= widgets::slider(ui, "Saturation", &mut self.saturation, -1.0..=1.0);
        changed |= widgets::slider(ui, "Lightness", &mut self.lightness, -1.0..=1.0);
        changed
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        let hue_shift = self.hue / 360.0;
        par_pixels(image, |px| {
            let r = px[0] as f32 / 255.0;
            let g = px[1] as f32 / 255.0;
            let b = px[2] as f32 / 255.0;

            let (mut h, mut s, mut l) = color::rgb_to_hsl(r, g, b);
            h = (h + hue_shift).rem_euclid(1.0);
            s = (s * (1.0 + self.saturation)).clamp(0.0, 1.0);
            l = (l + self.lightness).clamp(0.0, 1.0);

            let (nr, ng, nb) = color::hsl_to_rgb(h, s, l);
            px[0] = (nr.clamp(0.0, 1.0) * 255.0).round() as u8;
            px[1] = (ng.clamp(0.0, 1.0) * 255.0).round() as u8;
            px[2] = (nb.clamp(0.0, 1.0) * 255.0).round() as u8;
        });
    }
}
