use eframe::egui;
use image::{imageops, Rgba, RgbaImage};
use rayon::prelude::*;

use crate::color;
use crate::modifier::Modifier;
use crate::widgets;

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
enum Sampling {
    Nearest,
    Bilinear,
    Bicubic,
    Lanczos,
    Min,
    Max,
    Detail,
}

impl Sampling {
    const ALL: [Sampling; 7] = [
        Sampling::Nearest,
        Sampling::Bilinear,
        Sampling::Bicubic,
        Sampling::Lanczos,
        Sampling::Min,
        Sampling::Max,
        Sampling::Detail,
    ];

    fn name(self) -> &'static str {
        match self {
            Sampling::Nearest => "Nearest",
            Sampling::Bilinear => "Bilinear",
            Sampling::Bicubic => "Bicubic",
            Sampling::Lanczos => "Lanczos",
            Sampling::Min => "Min",
            Sampling::Max => "Max",
            Sampling::Detail => "Detail-Preserving",
        }
    }
}

fn default_detail() -> f32 {
    1.0
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Resize {
    width: u32,
    height: u32,
    sampling: Sampling,
    min_threshold: f32,
    max_threshold: f32,
    #[serde(default = "default_detail")]
    detail: f32,
}

impl Default for Resize {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            sampling: Sampling::Bicubic,
            min_threshold: 1.0,
            max_threshold: 0.0,
            detail: default_detail(),
        }
    }
}

impl Modifier for Resize {
    crate::modifier_serde!("resize");

    fn name(&self) -> &'static str {
        "Resize"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn on_added(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        if !ui.is_enabled() {
            ui.label(format!("Width: {}", self.width));
            ui.label(format!("Height: {}", self.height));
            ui.label(format!("Sampling: {}", self.sampling.name()));
            match self.sampling {
                Sampling::Min => ui.label(format!("Threshold: {:.2}", self.min_threshold)),
                Sampling::Max => ui.label(format!("Threshold: {:.2}", self.max_threshold)),
                Sampling::Detail => ui.label(format!("Detail: {:.2}", self.detail)),
                _ => ui.label(""),
            };
            return false;
        }

        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Width");
            changed |= widgets::drag_value(ui, &mut self.width, 1..=16384);
        });
        ui.horizontal(|ui| {
            ui.label("Height");
            changed |= widgets::drag_value(ui, &mut self.height, 1..=16384);
        });
        ui.separator();
        let combo = egui::ComboBox::from_id_source("resize_sampling")
            .selected_text(self.sampling.name())
            .show_ui(ui, |ui| {
                for sampling in Sampling::ALL {
                    changed |= ui
                        .selectable_value(&mut self.sampling, sampling, sampling.name())
                        .changed();
                }
            });
        changed |= widgets::combo_scroll(ui, &combo.response, &mut self.sampling, &Sampling::ALL);
        match self.sampling {
            Sampling::Min => {
                changed |= widgets::slider(ui, "Threshold", &mut self.min_threshold, 0.0..=1.0);
            }
            Sampling::Max => {
                changed |= widgets::slider(ui, "Threshold", &mut self.max_threshold, 0.0..=1.0);
            }
            Sampling::Detail => {
                changed |= widgets::slider(ui, "Detail", &mut self.detail, 0.0..=4.0);
            }
            _ => {}
        }
        changed
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        let width = self.width.max(1);
        let height = self.height.max(1);
        if width == image.width() && height == image.height() {
            return;
        }
        *image = match self.sampling {
            Sampling::Nearest => {
                imageops::resize(image, width, height, imageops::FilterType::Nearest)
            }
            Sampling::Bilinear => {
                imageops::resize(image, width, height, imageops::FilterType::Triangle)
            }
            Sampling::Bicubic => {
                imageops::resize(image, width, height, imageops::FilterType::CatmullRom)
            }
            Sampling::Lanczos => {
                imageops::resize(image, width, height, imageops::FilterType::Lanczos3)
            }
            Sampling::Min => resample_extreme(image, width, height, false, self.min_threshold),
            Sampling::Max => resample_extreme(image, width, height, true, self.max_threshold),
            Sampling::Detail => resample_detail(image, width, height, self.detail),
        };
    }

    fn gpu_pass(&self) -> Option<crate::gpu::GpuPass> {
        let (mode, param) = match self.sampling {
            Sampling::Nearest => (0.0, 0.0),
            Sampling::Bilinear => (1.0, 0.0),
            Sampling::Bicubic => (2.0, 0.0),
            Sampling::Lanczos => (3.0, 0.0),
            Sampling::Min => (4.0, self.min_threshold),
            Sampling::Max => (5.0, self.max_threshold),
            Sampling::Detail => (6.0, self.detail),
        };
        Some(
            crate::gpu::GpuPass::new("resize", RESIZE_SHADER)
                .with_uniforms(&crate::gpu::uniforms(&[
                    mode,
                    self.width.max(1) as f32,
                    self.height.max(1) as f32,
                    param,
                ]))
                .with_out_size(crate::gpu::OutSize::Fixed(self.width.max(1), self.height.max(1))),
        )
    }
}

const RESIZE_SHADER: &str = r#"
struct P { v: array<vec4<f32>, 1> };
@group(0) @binding(2) var<uniform> p: P;

fn lum01(c: vec3<f32>) -> f32 { return dot(c, vec3<f32>(0.299, 0.587, 0.114)); }

fn sinc(x: f32) -> f32 {
    if (abs(x) < 1e-6) { return 1.0; }
    let pix = 3.14159265 * x;
    return sin(pix) / pix;
}

// Filter kernel weight for the standard modes (1=triangle, 2=catmull, 3=lanczos3).
fn kweight(mode: i32, t: f32) -> f32 {
    let a = abs(t);
    if (mode == 1) {
        return max(0.0, 1.0 - a);
    } else if (mode == 2) {
        // Catmull-Rom (Keys a = -0.5).
        if (a < 1.0) {
            return 1.5 * a * a * a - 2.5 * a * a + 1.0;
        } else if (a < 2.0) {
            return -0.5 * a * a * a + 2.5 * a * a - 4.0 * a + 2.0;
        }
        return 0.0;
    } else {
        if (a < 3.0) { return sinc(t) * sinc(t / 3.0); }
        return 0.0;
    }
}

fn load_clamp(c: vec2<i32>, dim: vec2<i32>) -> vec4<f32> {
    return textureLoad(tex, clamp(c, vec2<i32>(0), dim - 1), 0);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let src = vec2<i32>(textureDimensions(tex));
    let dst = vec2<i32>(i32(p.v[0].y), i32(p.v[0].z));
    let mode = i32(p.v[0].x);
    let param = p.v[0].w;
    let oc = vec2<i32>(in.pos.xy);

    let scale = vec2<f32>(src) / vec2<f32>(dst);

    if (mode == 0) {
        // Nearest: source pixel at the destination cell center.
        let s = vec2<i32>(floor((vec2<f32>(oc) + 0.5) * scale));
        return load_clamp(s, src);
    }

    if (mode == 4 || mode == 5) {
        // Min / Max luminance within the source block (with threshold + nearest
        // fallback), mirroring resample_extreme.
        let x0 = (oc.x * src.x) / dst.x;
        let x1 = min(max(((oc.x + 1) * src.x + dst.x - 1) / dst.x, x0 + 1), src.x);
        let y0 = (oc.y * src.y) / dst.y;
        let y1 = min(max(((oc.y + 1) * src.y + dst.y - 1) / dst.y, y0 + 1), src.y);
        let take_max = mode == 5;
        let thr = param;
        var best = vec4<f32>(0.0);
        var best_l = select(1e9, -1e9, take_max);
        var found = false;
        for (var yy = y0; yy < y1; yy = yy + 1) {
            for (var xx = x0; xx < x1; xx = xx + 1) {
                let px = textureLoad(tex, vec2<i32>(xx, yy), 0);
                let l = lum01(px.rgb);
                let passes = select(l <= thr, l >= thr, take_max);
                if (passes) {
                    let better = select(l < best_l, l > best_l, take_max);
                    if (!found || better) { best = px; best_l = l; found = true; }
                }
            }
        }
        if (found) { return best; }
        let nx = min(((oc.x * 2 + 1) * src.x) / (dst.x * 2), src.x - 1);
        let ny = min(((oc.y * 2 + 1) * src.y) / (dst.y * 2), src.y - 1);
        return textureLoad(tex, vec2<i32>(nx, ny), 0);
    }

    if (mode == 6) {
        // Detail-preserving: weight each block pixel by its distance from the
        // block mean (mirrors resample_detail).
        let x0 = (oc.x * src.x) / dst.x;
        let x1 = min(max(((oc.x + 1) * src.x + dst.x - 1) / dst.x, x0 + 1), src.x);
        let y0 = (oc.y * src.y) / dst.y;
        let y1 = min(max(((oc.y + 1) * src.y + dst.y - 1) / dst.y, y0 + 1), src.y);
        var mean = vec4<f32>(0.0);
        var count = 0.0;
        for (var yy = y0; yy < y1; yy = yy + 1) {
            for (var xx = x0; xx < x1; xx = xx + 1) {
                mean = mean + textureLoad(tex, vec2<i32>(xx, yy), 0) * 255.0;
                count = count + 1.0;
            }
        }
        mean = mean / max(count, 1.0);
        var acc = vec4<f32>(0.0);
        var wsum = 0.0;
        for (var yy = y0; yy < y1; yy = yy + 1) {
            for (var xx = x0; xx < x1; xx = xx + 1) {
                let px = textureLoad(tex, vec2<i32>(xx, yy), 0) * 255.0;
                let d = length(px.rgb - mean.rgb);
                let w = pow(d, param);
                acc = acc + w * px;
                wsum = wsum + w;
            }
        }
        if (wsum > 1e-4) { return (acc / wsum) / 255.0; }
        return mean / 255.0;
    }

    // Standard filtered resample (bilinear / bicubic / lanczos), with the kernel
    // widened on downscale so it averages the footprint instead of aliasing.
    let support = f32(select(select(3, 2, mode == 2), 1, mode == 1));
    let center = (vec2<f32>(oc) + 0.5) * scale - 0.5;
    let rad = support * max(vec2<f32>(1.0), scale);
    let inv = 1.0 / max(vec2<f32>(1.0), scale);
    let lo = vec2<i32>(floor(center - rad));
    let hi = vec2<i32>(ceil(center + rad));
    var acc = vec4<f32>(0.0);
    var wsum = 0.0;
    for (var yy = lo.y; yy <= hi.y; yy = yy + 1) {
        let wy = kweight(mode, (f32(yy) - center.y) * inv.y);
        for (var xx = lo.x; xx <= hi.x; xx = xx + 1) {
            let wx = kweight(mode, (f32(xx) - center.x) * inv.x);
            let w = wx * wy;
            acc = acc + w * load_clamp(vec2<i32>(xx, yy), src);
            wsum = wsum + w;
        }
    }
    if (abs(wsum) < 1e-6) { return load_clamp(oc, src); }
    return acc / wsum;
}
"#;

fn luminance(pixel: &Rgba<u8>) -> f32 {
    color::luma(pixel[0] as f32, pixel[1] as f32, pixel[2] as f32)
}

/// Detail-preserving downscale (Weber et al. 2016). Each output pixel is a
/// weighted average of its source span, with weights set by how far each source
/// pixel deviates from the span's mean color: thin high-contrast features
/// (seams, grout - dark or light) dominate and survive, while flat regions
/// stay true to their average. `lambda` controls strength (0 = plain box
/// average; higher = stronger detail preservation).
fn resample_detail(src: &RgbaImage, dst_w: u32, dst_h: u32, lambda: f32) -> RgbaImage {
    let (src_w, src_h) = (src.width(), src.height());
    let mut out = RgbaImage::new(dst_w, dst_h);

    let span = |index: u32, dst: u32, src: u32| {
        let lo = (index as u64 * src as u64 / dst as u64) as u32;
        let hi = ((index as u64 + 1) * src as u64).div_ceil(dst as u64) as u32;
        (lo, hi.max(lo + 1).min(src))
    };

    let row_len = dst_w as usize * 4;
    let buffer: &mut [u8] = &mut out;
    buffer
        .par_chunks_mut(row_len)
        .enumerate()
        .for_each(|(oy, row)| {
            let oy = oy as u32;
            let (y0, y1) = span(oy, dst_h, src_h);
            for ox in 0..dst_w {
                let (x0, x1) = span(ox, dst_w, src_w);

                let mut mean = [0.0f32; 4];
                let mut count = 0.0f32;
                for yy in y0..y1 {
                    for xx in x0..x1 {
                        let p = src.get_pixel(xx, yy);
                        for c in 0..4 {
                            mean[c] += p[c] as f32;
                        }
                        count += 1.0;
                    }
                }
                for c in &mut mean {
                    *c /= count.max(1.0);
                }

                // Weight each pixel by its RGB distance from the span mean.
                let mut acc = [0.0f32; 4];
                let mut wsum = 0.0f32;
                for yy in y0..y1 {
                    for xx in x0..x1 {
                        let p = src.get_pixel(xx, yy);
                        let dist = ((p[0] as f32 - mean[0]).powi(2)
                            + (p[1] as f32 - mean[1]).powi(2)
                            + (p[2] as f32 - mean[2]).powi(2))
                        .sqrt();
                        let w = dist.powf(lambda);
                        wsum += w;
                        for c in 0..4 {
                            acc[c] += w * p[c] as f32;
                        }
                    }
                }

                let o = ox as usize * 4;
                for c in 0..4 {
                    // Flat span (all weights ~0): fall back to the mean.
                    let v = if wsum > 1e-4 { acc[c] / wsum } else { mean[c] };
                    row[o + c] = v.round().clamp(0.0, 255.0) as u8;
                }
            }
        });

    out
}

fn resample_extreme(
    src: &RgbaImage,
    dst_w: u32,
    dst_h: u32,
    take_max: bool,
    threshold: f32,
) -> RgbaImage {
    let (src_w, src_h) = (src.width(), src.height());
    let mut out = RgbaImage::new(dst_w, dst_h);
    let threshold = threshold * 255.0;

    let span = |index: u32, dst: u32, src: u32| {
        let lo = (index as u64 * src as u64 / dst as u64) as u32;
        let hi = ((index as u64 + 1) * src as u64).div_ceil(dst as u64) as u32;
        (lo, hi.max(lo + 1).min(src))
    };

    let nearest = |index: u32, dst: u32, src: u32| {
        (((index as u64 * 2 + 1) * src as u64) / (dst as u64 * 2)).min(src as u64 - 1) as u32
    };

    let row_len = dst_w as usize * 4;
    let buffer: &mut [u8] = &mut out;
    buffer
        .par_chunks_mut(row_len)
        .enumerate()
        .for_each(|(oy, row)| {
            let oy = oy as u32;
            let (y0, y1) = span(oy, dst_h, src_h);
            let ny = nearest(oy, dst_h, src_h);
            for ox in 0..dst_w {
                let (x0, x1) = span(ox, dst_w, src_w);
                let mut eligible: Option<(Rgba<u8>, f32)> = None;
                for yy in y0..y1 {
                    for xx in x0..x1 {
                        let pixel = src.get_pixel(xx, yy);
                        let lum = luminance(pixel);
                        let passes = if take_max {
                            lum >= threshold
                        } else {
                            lum <= threshold
                        };
                        if !passes {
                            continue;
                        }
                        let better = match eligible {
                            None => true,
                            Some((_, best_lum)) => {
                                if take_max {
                                    lum > best_lum
                                } else {
                                    lum < best_lum
                                }
                            }
                        };
                        if better {
                            eligible = Some((*pixel, lum));
                        }
                    }
                }
                let pixel = eligible
                    .map(|(p, _)| p)
                    .unwrap_or_else(|| *src.get_pixel(nearest(ox, dst_w, src_w), ny));
                let o = ox as usize * 4;
                row[o..o + 4].copy_from_slice(&pixel.0);
            }
        });

    out
}
