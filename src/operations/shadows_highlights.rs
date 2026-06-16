use eframe::egui;

use crate::operation::Operation;
use crate::widgets;

#[derive(Default)]
pub struct ShadowsHighlights {
    shadows: f32,
    highlights: f32,
}

impl Operation for ShadowsHighlights {
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
        for pixel in image.pixels_mut() {
            let lum = (0.299 * pixel[0] as f32 + 0.587 * pixel[1] as f32 + 0.114 * pixel[2] as f32)
                / 255.0;
            let shadow_mask = (1.0 - lum).powi(2);
            let highlight_mask = lum.powi(2);

            for channel in &mut pixel.0[..3] {
                let mut value = *channel as f32 / 255.0;
                value += self.shadows * shadow_mask * (1.0 - value);
                value -= self.highlights * highlight_mask * value;
                *channel = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
    }
}
