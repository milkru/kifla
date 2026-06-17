use eframe::egui;
use image::RgbaImage;
use rayon::prelude::*;

use crate::operation::Operation;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Repeat {
    x: u32,
    y: u32,
}

impl Default for Repeat {
    fn default() -> Self {
        Self { x: 1, y: 1 }
    }
}

impl Operation for Repeat {
    crate::op_serde!("repeat");

    fn name(&self) -> &'static str {
        "Repeat"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        if !ui.is_enabled() {
            ui.label(format!("X: {}", self.x));
            ui.label(format!("Y: {}", self.y));
            return false;
        }
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("X");
            let r = ui.add(egui::DragValue::new(&mut self.x).clamp_range(1..=32));
            changed |= r.drag_released() || r.lost_focus();
        });
        ui.horizontal(|ui| {
            ui.label("Y");
            let r = ui.add(egui::DragValue::new(&mut self.y).clamp_range(1..=32));
            changed |= r.drag_released() || r.lost_focus();
        });
        changed
    }

    fn apply(&self, image: &mut RgbaImage) {
        let (rx, ry) = (self.x.clamp(1, 32), self.y.clamp(1, 32));
        if rx == 1 && ry == 1 {
            return;
        }
        let (w, h) = (image.width(), image.height());
        if w == 0 || h == 0 {
            return;
        }

        // Tile the texture rx×ry times within the same canvas: each output pixel
        // averages the source block it maps to (box downsample), keeping it crisp
        // and seamless without changing resolution.
        let src = image.clone();
        let mut out = RgbaImage::new(w, h);
        let row_len = w as usize * 4;
        let sxn = rx.min(4);
        let syn = ry.min(4);
        let dst: &mut [u8] = &mut out;

        dst.par_chunks_mut(row_len)
            .enumerate()
            .for_each(|(oy, orow)| {
                let oy = oy as u32;
                for ox in 0..w {
                    let mut acc = [0u32; 4];
                    let mut count = 0u32;
                    for j in 0..syn {
                        let sy = (oy * ry + j * ry / syn) % h;
                        for i in 0..sxn {
                            let sx = (ox * rx + i * rx / sxn) % w;
                            let p = src.get_pixel(sx, sy);
                            for c in 0..4 {
                                acc[c] += p[c] as u32;
                            }
                            count += 1;
                        }
                    }
                    let o = ox as usize * 4;
                    for c in 0..4 {
                        orow[o + c] = (acc[c] / count) as u8;
                    }
                }
            });

        *image = out;
    }
}
