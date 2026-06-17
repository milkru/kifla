use color_quant::NeuQuant;
use eframe::egui;
use image::{Rgba, RgbaImage};

use crate::operation::{par_pixels, Operation};
use crate::widgets;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct IndexedColor {
    colors: u32,
    dither: bool,
    amount: f32,
}

impl Default for IndexedColor {
    fn default() -> Self {
        Self {
            colors: 256,
            dither: false,
            amount: 1.0,
        }
    }
}

impl Operation for IndexedColor {
    crate::op_serde!("indexed_color");

    fn name(&self) -> &'static str {
        "Indexed Color"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        if !ui.is_enabled() {
            ui.label(format!("Colors: {}", self.colors));
            ui.label(format!("Dither: {}", if self.dither { "on" } else { "off" }));
            if self.dither {
                ui.label(format!("Amount: {:.2}", self.amount));
            }
            return false;
        }

        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Colors");
            let r = ui.add(egui::DragValue::new(&mut self.colors).clamp_range(2..=256));
            changed |= r.drag_released() || r.lost_focus();
        });
        changed |= ui.checkbox(&mut self.dither, "Dither").changed();
        if self.dither {
            changed |= widgets::slider(ui, "Amount", &mut self.amount, 0.0..=1.0);
        }
        changed
    }

    fn apply(&self, image: &mut RgbaImage) {
        let n = self.colors.clamp(2, 256) as usize;
        if (n >= 256 && !self.dither) || image.width() == 0 || image.height() == 0 {
            return;
        }

        let nq = NeuQuant::new(10, n, image.as_raw());
        let palette = nq.color_map_rgba();

        if self.dither {
            dither_floyd_steinberg(image, &nq, &palette, self.amount);
        } else {
            par_pixels(image, |px| {
                let i = nq.index_of(px) * 4;
                px[0] = palette[i];
                px[1] = palette[i + 1];
                px[2] = palette[i + 2];
            });
        }
    }
}

fn dither_floyd_steinberg(image: &mut RgbaImage, nq: &NeuQuant, palette: &[u8], amount: f32) {
    let (w, h) = (image.width() as i32, image.height() as i32);
    let mut buf: Vec<[f32; 3]> = image
        .pixels()
        .map(|p| [p[0] as f32, p[1] as f32, p[2] as f32])
        .collect();

    let clamp = |v: f32| v.round().clamp(0.0, 255.0) as u8;

    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            let old = buf[idx];
            let alpha = image.get_pixel(x as u32, y as u32)[3];
            let probe = [clamp(old[0]), clamp(old[1]), clamp(old[2]), alpha];
            let p = nq.index_of(&probe) * 4;
            let new = [palette[p] as f32, palette[p + 1] as f32, palette[p + 2] as f32];
            image.put_pixel(x as u32, y as u32, Rgba([palette[p], palette[p + 1], palette[p + 2], alpha]));

            let err = [
                (old[0] - new[0]) * amount,
                (old[1] - new[1]) * amount,
                (old[2] - new[2]) * amount,
            ];
            let mut spread = |nx: i32, ny: i32, f: f32| {
                if nx >= 0 && nx < w && ny >= 0 && ny < h {
                    let n = (ny * w + nx) as usize;
                    buf[n][0] += err[0] * f;
                    buf[n][1] += err[1] * f;
                    buf[n][2] += err[2] * f;
                }
            };
            spread(x + 1, y, 7.0 / 16.0);
            spread(x - 1, y + 1, 3.0 / 16.0);
            spread(x, y + 1, 5.0 / 16.0);
            spread(x + 1, y + 1, 1.0 / 16.0);
        }
    }
}
