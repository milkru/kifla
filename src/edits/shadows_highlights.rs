use eframe::egui;

use crate::color;
use crate::edit::Edit;
use crate::pixel::par_pixels;
use crate::widgets;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct ShadowsHighlights {
    shadows: f32,
    highlights: f32,
}

impl Edit for ShadowsHighlights {
    crate::edit_serde!("shadows_highlights");

    fn name(&self) -> &'static str {
        "Shadows / Highlights"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        changed |= widgets::slider(ui, "Shadows", &mut self.shadows, 0.0..=1.0);
        changed |= widgets::slider(ui, "Highlights", &mut self.highlights, 0.0..=1.0);
        changed
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        par_pixels(image, |px| {
            let lum = color::luma(px[0] as f32, px[1] as f32, px[2] as f32) / 255.0;
            let shadow_mask = (1.0 - lum).powi(2);
            let highlight_mask = lum.powi(2);

            for channel in &mut px[..3] {
                let mut value = *channel as f32 / 255.0;
                value += self.shadows * shadow_mask * (1.0 - value);
                value -= self.highlights * highlight_mask * value;
                *channel = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        });
    }
}
