use eframe::egui;
use image::RgbaImage;
use rayon::prelude::*;

use crate::edit::Edit;
use crate::widgets;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct MakeSeamless {
    falloff: f32,
    overlap_x: f32,
    overlap_y: f32,
}

impl Default for MakeSeamless {
    fn default() -> Self {
        Self {
            falloff: 0.5,
            overlap_x: 0.0,
            overlap_y: 0.0,
        }
    }
}

impl Edit for MakeSeamless {
    crate::edit_serde!("make_seamless");

    fn name(&self) -> &'static str {
        "Make Seamless"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        if !ui.is_enabled() {
            ui.label(format!("Falloff: {:.2}", self.falloff));
            ui.label(format!("Overlap X: {:.2}", self.overlap_x));
            ui.label(format!("Overlap Y: {:.2}", self.overlap_y));
            return false;
        }
        let mut changed = false;
        changed |= widgets::slider(ui, "Edge Falloff", &mut self.falloff, 0.01..=1.0);
        changed |= widgets::slider(ui, "Overlap X", &mut self.overlap_x, 0.0..=1.0);
        changed |= widgets::slider(ui, "Overlap Y", &mut self.overlap_y, 0.0..=1.0);
        changed
    }

    fn apply(&self, image: &mut RgbaImage) {
        let (w, h) = (image.width(), image.height());
        if (self.overlap_x <= 0.0 && self.overlap_y <= 0.0) || w < 2 || h < 2 {
            return;
        }

        let src = image.clone();
        let (ow, oh) = (w / 2, h / 2);
        let band_x = self.overlap_x * 0.5 * w as f32;
        let band_y = self.overlap_y * 0.5 * h as f32;
        let pow = 1.0 / self.falloff.clamp(0.01, 1.0);
        let row_len = w as usize * 4;

        let buffer: &mut [u8] = image;
        buffer
            .par_chunks_mut(row_len)
            .enumerate()
            .for_each(|(y, row)| {
                let y = y as u32;
                let dye = y.min(h - 1 - y) as f32;
                let wy = if band_y > 0.0 && dye < band_y {
                    (1.0 - dye / band_y).powf(pow)
                } else {
                    0.0
                };
                for x in 0..w {
                    let dxe = x.min(w - 1 - x) as f32;
                    let wx = if band_x > 0.0 && dxe < band_x {
                        (1.0 - dxe / band_x).powf(pow)
                    } else {
                        0.0
                    };
                    let t = wx.max(wy).clamp(0.0, 1.0);
                    let orig = src.get_pixel(x, y);
                    let shift = src.get_pixel((x + ow) % w, (y + oh) % h);
                    let o = x as usize * 4;
                    for c in 0..4 {
                        let v = orig[c] as f32 * (1.0 - t) + shift[c] as f32 * t;
                        row[o + c] = v.round().clamp(0.0, 255.0) as u8;
                    }
                }
            });
    }
}
