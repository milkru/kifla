use eframe::egui;
use rayon::prelude::*;

use crate::color;
use crate::modifier::Modifier;
use crate::widgets;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Lighting {
    amount: f32,
}

impl Modifier for Lighting {
    crate::modifier_serde!("lighting");

    fn name(&self) -> &'static str {
        "Lighting Normalization"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        widgets::slider(ui, "Amount", &mut self.amount, 0.0..=3.0)
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        let amount = self.amount.clamp(0.0, 3.0);
        if amount <= 0.0 {
            return;
        }
        let (w, h) = (image.width() as usize, image.height() as usize);
        if w == 0 || h == 0 {
            return;
        }

        // Estimate the low-frequency illumination from a heavily blurred
        // luminance field, then divide it out so broad shading flattens while
        // the texture detail (the sharp, high-frequency part) is preserved.
        let src: &[u8] = image;
        let mut lum = vec![0.0f32; w * h];
        lum.par_iter_mut().enumerate().for_each(|(i, l)| {
            let o = i * 4;
            *l = color::luma(src[o] as f32, src[o + 1] as f32, src[o + 2] as f32);
        });

        let mean = lum.par_iter().sum::<f32>() / (w * h) as f32;
        let radius = (w.min(h) / 4).max(1);
        let illum = box_blur_wrap(&lum, w, h, radius, 3);

        let buffer: &mut [u8] = image;
        buffer.par_chunks_mut(4).enumerate().for_each(|(i, px)| {
            let gain = (mean / illum[i].max(1e-3)).clamp(0.2, 5.0);
            // Keep the gain non-negative so strong amounts flatten rather than
            // invert bright regions (which reads as contour banding).
            let gain = (1.0 + (gain - 1.0) * amount).max(0.0);
            for channel in &mut px[..3] {
                *channel = (*channel as f32 * gain).round().clamp(0.0, 255.0) as u8;
            }
        });
    }
}

// Wrapped (tiling) box blur of a single-channel field, separable and applied
// `passes` times to approximate a Gaussian. The running sum keeps each pass
// O(w·h) regardless of radius.
fn box_blur_wrap(src: &[f32], w: usize, h: usize, radius: usize, passes: usize) -> Vec<f32> {
    let rh = radius.min((w.max(2) - 1) / 2);
    let rv = radius.min((h.max(2) - 1) / 2);
    let mut cur = src.to_vec();
    let mut a = vec![0.0f32; w * h];
    let mut b = vec![0.0f32; w * h];
    let mut out = vec![0.0f32; w * h];
    for _ in 0..passes {
        blur_lines(&cur, w, rh, &mut a);
        transpose(&a, w, h, &mut b);
        blur_lines(&b, h, rv, &mut a);
        transpose(&a, h, w, &mut out);
        std::mem::swap(&mut cur, &mut out);
    }
    cur
}

// Box blur with wraparound along each row of width `width`.
fn blur_lines(src: &[f32], width: usize, r: usize, dst: &mut [f32]) {
    if r == 0 {
        dst.copy_from_slice(src);
        return;
    }
    let win = (2 * r + 1) as f32;
    let wi = width as isize;
    dst.par_chunks_mut(width)
        .zip(src.par_chunks(width))
        .for_each(|(out, row)| {
            let mut sum = 0.0;
            for k in 0..=2 * r {
                let idx = (k as isize - r as isize).rem_euclid(wi) as usize;
                sum += row[idx];
            }
            for x in 0..width {
                out[x] = sum / win;
                let drop = (x as isize - r as isize).rem_euclid(wi) as usize;
                let add = (x as isize + r as isize + 1).rem_euclid(wi) as usize;
                sum += row[add] - row[drop];
            }
        });
}

// Transpose an h×w field into a w×h one.
fn transpose(src: &[f32], w: usize, h: usize, dst: &mut [f32]) {
    dst.par_chunks_mut(h).enumerate().for_each(|(x, col)| {
        for y in 0..h {
            col[y] = src[y * w + x];
        }
    });
}
