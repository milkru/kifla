use eframe::egui;
use image::RgbaImage;
use rayon::prelude::*;

use crate::modifier::Modifier;
use crate::widgets;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Repeat {
    x: f32,
    y: f32,
}

impl Default for Repeat {
    fn default() -> Self {
        Self { x: 1.0, y: 1.0 }
    }
}

impl Modifier for Repeat {
    crate::modifier_serde!("repeat");

    fn name(&self) -> &'static str {
        "Repeat"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        changed |= widgets::slider(ui, "X", &mut self.x, 1.0..=32.0);
        changed |= widgets::slider(ui, "Y", &mut self.y, 1.0..=32.0);
        changed
    }

    fn apply(&self, image: &mut RgbaImage) {
        let (rx, ry) = (self.x.clamp(1.0, 32.0), self.y.clamp(1.0, 32.0));
        if (rx - 1.0).abs() < 1e-4 && (ry - 1.0).abs() < 1e-4 {
            return;
        }
        let (w, h) = (image.width(), image.height());
        if w == 0 || h == 0 {
            return;
        }

        // Tile the texture rx×ry times within the same canvas: each output pixel
        // averages the source block it maps to (box downsample), keeping it crisp
        // without changing resolution. Whole-number counts wrap seamlessly;
        // fractional ones do not tile perfectly at the canvas edges.
        let src = image.clone();
        let mut out = RgbaImage::new(w, h);
        let row_len = w as usize * 4;
        let sxn = (rx.ceil() as u32).clamp(1, 4);
        let syn = (ry.ceil() as u32).clamp(1, 4);
        let dst: &mut [u8] = &mut out;

        dst.par_chunks_mut(row_len)
            .enumerate()
            .for_each(|(oy, orow)| {
                let oy = oy as f32;
                for ox in 0..w {
                    let mut acc = [0u32; 4];
                    let mut count = 0u32;
                    for j in 0..syn {
                        let fy = oy * ry + j as f32 * ry / syn as f32;
                        let sy = (fy.floor() as i64).rem_euclid(h as i64) as u32;
                        for i in 0..sxn {
                            let fx = ox as f32 * rx + i as f32 * rx / sxn as f32;
                            let sx = (fx.floor() as i64).rem_euclid(w as i64) as u32;
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

    fn gpu_pass(&self) -> Option<crate::gpu::GpuPass> {
        Some(
            crate::gpu::GpuPass::new(
                "repeat",
                r#"
struct P { v: array<vec4<f32>, 1> };
@group(0) @binding(2) var<uniform> p: P;
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let dim = vec2<i32>(textureDimensions(tex));
    let rx = clamp(p.v[0].x, 1.0, 32.0);
    let ry = clamp(p.v[0].y, 1.0, 32.0);
    let sxn = u32(clamp(ceil(rx), 1.0, 4.0));
    let syn = u32(clamp(ceil(ry), 1.0, 4.0));
    let oc = floor(in.pos.xy);
    // Accumulate in 0..255 integer space and floor the mean, mirroring the CPU
    // path's integer averaging (so flat regions match exactly, no rounding drift).
    var acc = vec4<f32>(0.0);
    var count = 0.0;
    for (var j = 0u; j < syn; j = j + 1u) {
        let fy = oc.y * ry + f32(j) * ry / f32(syn);
        let sy = ((i32(floor(fy)) % dim.y) + dim.y) % dim.y;
        for (var i = 0u; i < sxn; i = i + 1u) {
            let fx = oc.x * rx + f32(i) * rx / f32(sxn);
            let sx = ((i32(floor(fx)) % dim.x) + dim.x) % dim.x;
            acc = acc + round(textureLoad(tex, vec2<i32>(sx, sy), 0) * 255.0);
            count = count + 1.0;
        }
    }
    return floor(acc / count) / 255.0;
}
"#,
            )
            .with_uniforms(&crate::gpu::uniforms(&[self.x, self.y])),
        )
    }
}
