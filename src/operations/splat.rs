use eframe::egui;
use image::{Rgba, RgbaImage};

use crate::operation::Operation;
use crate::widgets;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Splat {
    count: u32,
    scale: f32,
    rotation: f32,
    random_rotation: f32,
    wobble: f32,
    seed: u32,
}

impl Default for Splat {
    fn default() -> Self {
        Self {
            count: 0,
            scale: 1.0,
            rotation: 0.0,
            random_rotation: 0.5,
            wobble: 0.5,
            seed: 0,
        }
    }
}

fn rng(state: &mut u32) -> f32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

fn sample(src: &RgbaImage, x: f32, y: f32, w: u32, h: u32) -> [f32; 4] {
    let xf = x.rem_euclid(w as f32);
    let yf = y.rem_euclid(h as f32);
    let x0 = (xf.floor() as u32) % w;
    let y0 = (yf.floor() as u32) % h;
    let x1 = (x0 + 1) % w;
    let y1 = (y0 + 1) % h;
    let tx = xf - xf.floor();
    let ty = yf - yf.floor();
    let p = [
        src.get_pixel(x0, y0),
        src.get_pixel(x1, y0),
        src.get_pixel(x0, y1),
        src.get_pixel(x1, y1),
    ];
    let mut out = [0.0; 4];
    for c in 0..4 {
        let a = p[0][c] as f32 * (1.0 - tx) + p[1][c] as f32 * tx;
        let b = p[2][c] as f32 * (1.0 - tx) + p[3][c] as f32 * tx;
        out[c] = a * (1.0 - ty) + b * ty;
    }
    out
}

impl Operation for Splat {
    crate::op_serde!("splat");

    fn name(&self) -> &'static str {
        "Splat"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        if !ui.is_enabled() {
            ui.label(format!("Count: {}", self.count));
            ui.label(format!("Scale: {:.2}", self.scale));
            ui.label(format!("Rotation: {:.2}", self.rotation));
            return false;
        }
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Count");
            let r = ui.add(egui::DragValue::new(&mut self.count).clamp_range(0..=64));
            changed |= r.drag_released() || r.lost_focus();
        });
        changed |= widgets::slider(ui, "Scale", &mut self.scale, 0.25..=2.0);
        changed |= widgets::slider(ui, "Rotation", &mut self.rotation, 0.0..=1.0);
        changed |= widgets::slider(ui, "Random Rotation", &mut self.random_rotation, 0.0..=1.0);
        changed |= widgets::slider(ui, "Wobble", &mut self.wobble, 0.0..=1.0);
        ui.horizontal(|ui| {
            ui.label(format!("Seed: {}", self.seed));
            if ui.button("🎲 Randomize").clicked() {
                self.seed = self.seed.wrapping_add(1);
                changed = true;
            }
        });
        changed
    }

    fn apply(&self, image: &mut RgbaImage) {
        let count = self.count.min(64);
        let (w, h) = (image.width(), image.height());
        if count == 0 || w == 0 || h == 0 {
            return;
        }

        let src = image.clone();
        let (wf, hf) = (w as f32, h as f32);
        let scale = self.scale.clamp(0.1, 4.0);
        let radius = 0.5 * wf.min(hf) * scale;
        let (cx0, cy0) = (wf * 0.5, hf * 0.5);
        let grid = (count as f32).sqrt().ceil().max(1.0) as u32;
        let (cell_w, cell_h) = (wf / grid as f32, hf / grid as f32);

        let mut idx = 0u32;
        'outer: for gy in 0..grid {
            for gx in 0..grid {
                if idx >= count {
                    break 'outer;
                }
                let mut state = self
                    .seed
                    .wrapping_mul(2_654_435_761)
                    .wrapping_add(idx.wrapping_mul(40_503))
                    | 1;
                let (r1, r2, r3) = (rng(&mut state), rng(&mut state), rng(&mut state));
                idx += 1;

                let px = (gx as f32 + 0.5) * cell_w + self.wobble * (r1 - 0.5) * cell_w;
                let py = (gy as f32 + 0.5) * cell_h + self.wobble * (r2 - 0.5) * cell_h;
                let angle =
                    (self.rotation + self.random_rotation * r3) * std::f32::consts::TAU;
                let (sa, ca) = angle.sin_cos();

                let min_x = (px - radius).floor() as i32;
                let max_x = (px + radius).ceil() as i32;
                let min_y = (py - radius).floor() as i32;
                let max_y = (py + radius).ceil() as i32;

                for sy in min_y..=max_y {
                    for sx in min_x..=max_x {
                        let dx = sx as f32 + 0.5 - px;
                        let dy = sy as f32 + 0.5 - py;
                        let dist = (dx * dx + dy * dy).sqrt();
                        if dist > radius {
                            continue;
                        }
                        let feather = {
                            let t = 1.0 - dist / radius;
                            t * t
                        };
                        let ux = (ca * dx + sa * dy) / scale;
                        let uy = (-sa * dx + ca * dy) / scale;
                        let s = sample(&src, cx0 + ux, cy0 + uy, w, h);
                        let ox = sx.rem_euclid(w as i32) as u32;
                        let oy = sy.rem_euclid(h as i32) as u32;
                        let base = image.get_pixel(ox, oy);
                        let mut out = [0u8; 4];
                        for c in 0..4 {
                            out[c] = (base[c] as f32 * (1.0 - feather) + s[c] * feather)
                                .round()
                                .clamp(0.0, 255.0) as u8;
                        }
                        image.put_pixel(ox, oy, Rgba(out));
                    }
                }
            }
        }
    }
}
