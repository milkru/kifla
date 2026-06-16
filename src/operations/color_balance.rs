use eframe::egui;

use crate::operation::Operation;
use crate::widgets;

#[derive(Default)]
pub struct ColorBalance {
    shadows: [f32; 3],
    midtones: [f32; 3],
    highlights: [f32; 3],
}

fn range_ui(ui: &mut egui::Ui, title: &str, amounts: &mut [f32; 3]) -> bool {
    ui.label(title);
    let mut changed = false;
    changed |= widgets::slider(ui, "Cyan-Red", &mut amounts[0], -1.0..=1.0);
    changed |= widgets::slider(ui, "Magenta-Green", &mut amounts[1], -1.0..=1.0);
    changed |= widgets::slider(ui, "Yellow-Blue", &mut amounts[2], -1.0..=1.0);
    changed
}

impl Operation for ColorBalance {
    fn name(&self) -> &'static str {
        "Color Balance"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        changed |= range_ui(ui, "Shadows", &mut self.shadows);
        changed |= range_ui(ui, "Midtones", &mut self.midtones);
        changed |= range_ui(ui, "Highlights", &mut self.highlights);
        changed
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        const STRENGTH: f32 = 0.5;
        for pixel in image.pixels_mut() {
            let rgb = [
                pixel[0] as f32 / 255.0,
                pixel[1] as f32 / 255.0,
                pixel[2] as f32 / 255.0,
            ];
            let lum = 0.299 * rgb[0] + 0.587 * rgb[1] + 0.114 * rgb[2];
            let shadow = (1.0 - 2.0 * lum).max(0.0);
            let highlight = (2.0 * lum - 1.0).max(0.0);
            let midtone = (1.0 - shadow - highlight).max(0.0);

            for c in 0..3 {
                let shift = STRENGTH
                    * (self.shadows[c] * shadow
                        + self.midtones[c] * midtone
                        + self.highlights[c] * highlight);
                pixel[c] = ((rgb[c] + shift).clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
    }
}
