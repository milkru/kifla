use eframe::egui;

use crate::color;
use crate::modifier::Modifier;
use crate::pixel::{par_pixels, to_u8};
use crate::widgets;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Vibrance {
    vibrance: f32,
    saturation: f32,
}

impl Modifier for Vibrance {
    crate::modifier_serde!("vibrance");

    fn name(&self) -> &'static str {
        "Vibrance"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        changed |= widgets::slider(ui, "Vibrance", &mut self.vibrance, -1.0..=1.0);
        changed |= widgets::slider(ui, "Saturation", &mut self.saturation, -1.0..=1.0);
        changed
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        par_pixels(image, |px| {
            let r = px[0] as f32 / 255.0;
            let g = px[1] as f32 / 255.0;
            let b = px[2] as f32 / 255.0;

            let (h, mut s, l) = color::rgb_to_hsl(r, g, b);
            s *= 1.0 + self.saturation;
            s += self.vibrance * (1.0 - s);
            s = s.clamp(0.0, 1.0);

            let (nr, ng, nb) = color::hsl_to_rgb(h, s, l);
            px[0] = to_u8(nr);
            px[1] = to_u8(ng);
            px[2] = to_u8(nb);
        });
    }
}
