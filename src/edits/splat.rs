use eframe::egui;
use image::RgbaImage;
use rayon::prelude::*;

use crate::edit::Edit;
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

// xorshift32, returning a deterministic 0..1 sample so splats are reproducible per seed.
fn rng(state: &mut u32) -> f32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

// Bilinear sample of a wrapped (tiling) source, indexing the raw buffer directly.
fn sample(src: &[u8], x: f32, y: f32, w: u32, h: u32) -> [f32; 4] {
    let xf = x.rem_euclid(w as f32);
    let yf = y.rem_euclid(h as f32);
    let x0 = (xf.floor() as u32) % w;
    let y0 = (yf.floor() as u32) % h;
    let x1 = (x0 + 1) % w;
    let y1 = (y0 + 1) % h;
    let tx = xf - xf.floor();
    let ty = yf - yf.floor();
    let idx = |x: u32, y: u32| ((y * w + x) as usize) * 4;
    let (i00, i10, i01, i11) = (idx(x0, y0), idx(x1, y0), idx(x0, y1), idx(x1, y1));
    let mut out = [0.0; 4];
    for c in 0..4 {
        let a = src[i00 + c] as f32 * (1.0 - tx) + src[i10 + c] as f32 * tx;
        let b = src[i01 + c] as f32 * (1.0 - tx) + src[i11 + c] as f32 * tx;
        out[c] = a * (1.0 - ty) + b * ty;
    }
    out
}

// One placed splat: center, rotation (sin/cos), and integer bounding box.
struct Stamp {
    px: f32,
    py: f32,
    sa: f32,
    ca: f32,
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
}

impl Edit for Splat {
    crate::edit_serde!("splat");

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
            changed |= widgets::drag_value(ui, &mut self.count, 0..=64);
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

        let (wf, hf) = (w as f32, h as f32);
        let scale = self.scale.clamp(0.1, 4.0);
        let radius = 0.5 * wf.min(hf) * scale;
        let r2 = radius * radius;
        let (cx0, cy0) = (wf * 0.5, hf * 0.5);
        let grid = (count as f32).sqrt().ceil().max(1.0) as u32;
        let (cell_w, cell_h) = (wf / grid as f32, hf / grid as f32);

        // Place every splat up front so the per-pixel pass can replay them in order.
        let stamps: Vec<Stamp> = (0..count)
            .map(|idx| {
                let (gx, gy) = (idx % grid, idx / grid);
                let mut state = self
                    .seed
                    .wrapping_mul(2_654_435_761)
                    .wrapping_add(idx.wrapping_mul(40_503))
                    | 1;
                let (r1, r2, r3) = (rng(&mut state), rng(&mut state), rng(&mut state));
                let px = (gx as f32 + 0.5) * cell_w + self.wobble * (r1 - 0.5) * cell_w;
                let py = (gy as f32 + 0.5) * cell_h + self.wobble * (r2 - 0.5) * cell_h;
                let angle = (self.rotation + self.random_rotation * r3) * std::f32::consts::TAU;
                let (sa, ca) = angle.sin_cos();
                Stamp {
                    px,
                    py,
                    sa,
                    ca,
                    min_x: (px - radius).floor() as i32,
                    max_x: (px + radius).ceil() as i32,
                    min_y: (py - radius).floor() as i32,
                    max_y: (py + radius).ceil() as i32,
                }
            })
            .collect();

        // Gather instead of scatter: each output pixel composites the stamps that
        // cover it (including wrapped copies) in stamp order — identical result to
        // the serial scatter, but parallel across rows.
        let src = image.clone();
        let src_raw: &[u8] = &src;
        let (wi, hi) = (w as i32, h as i32);
        let row_len = w as usize * 4;
        let buffer: &mut [u8] = image;
        buffer
            .par_chunks_mut(row_len)
            .enumerate()
            .for_each(|(oy, row)| {
                let oy = oy as i32;
                for ox in 0..wi {
                    let o = ox as usize * 4;
                    let mut color = [
                        row[o] as f32,
                        row[o + 1] as f32,
                        row[o + 2] as f32,
                        row[o + 3] as f32,
                    ];
                    for st in &stamps {
                        // Wrapped source coords (≡ ox/oy mod w/h) inside this stamp's box.
                        let kx0 = (st.min_x - ox + wi - 1).div_euclid(wi);
                        let kx1 = (st.max_x - ox).div_euclid(wi);
                        let ky0 = (st.min_y - oy + hi - 1).div_euclid(hi);
                        let ky1 = (st.max_y - oy).div_euclid(hi);
                        let mut sy = oy + ky0 * hi;
                        for _ in ky0..=ky1 {
                            let dy = sy as f32 + 0.5 - st.py;
                            let mut sx = ox + kx0 * wi;
                            for _ in kx0..=kx1 {
                                let dx = sx as f32 + 0.5 - st.px;
                                let d2 = dx * dx + dy * dy;
                                if d2 <= r2 {
                                    let t = 1.0 - d2.sqrt() / radius;
                                    let feather = t * t;
                                    let ux = (st.ca * dx + st.sa * dy) / scale;
                                    let uy = (-st.sa * dx + st.ca * dy) / scale;
                                    let s = sample(src_raw, cx0 + ux, cy0 + uy, w, h);
                                    for c in 0..4 {
                                        color[c] = (color[c] * (1.0 - feather) + s[c] * feather)
                                            .round()
                                            .clamp(0.0, 255.0);
                                    }
                                }
                                sx += wi;
                            }
                            sy += hi;
                        }
                    }
                    row[o] = color[0] as u8;
                    row[o + 1] = color[1] as u8;
                    row[o + 2] = color[2] as u8;
                    row[o + 3] = color[3] as u8;
                }
            });
    }
}
