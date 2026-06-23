use eframe::egui;

use crate::modifier::Modifier;
use crate::widgets;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct ColorNormalization {
    amount: f32,
    #[serde(default)]
    width: u32,
    #[serde(default)]
    height: u32,
}

impl Modifier for ColorNormalization {
    crate::modifier_serde!("color_normalization");

    fn name(&self) -> &'static str {
        "Color Normalization"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn on_added(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        widgets::slider(ui, "Amount", &mut self.amount, 0.0..=2.0)
    }

    fn gpu_passes(&self) -> Vec<crate::gpu::GpuPass> {
        use crate::gpu::{GpuPass, OutSize};
        if self.amount <= 0.0 {
            return Vec::new(); // no-op: input passes through unchanged
        }

        // Chroma field (YCbCr Cb/Cr), then a downsample pyramid approximating the
        // heavy blur (the low-frequency color cast), then a final pass that pushes
        // each pixel's chroma back toward the global average while keeping its
        // original luma. Same shape as Lighting Normalization, but subtractive in
        // chroma instead of a luma gain.
        let maxd = self.width.max(self.height).max(1) as f32;
        let maxd = if maxd <= 1.0 { 1024.0 } else { maxd };
        let depth = (maxd.log2().floor() as i32 - 2).clamp(1, 12) as usize;

        let mut passes = Vec::with_capacity(depth + 2);
        passes.push(GpuPass::new(
            "color_norm_chroma",
            r#"
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureLoad(tex, vec2<i32>(in.pos.xy), 0);
    // JFIF YCbCr chroma, stored 0.5-centered so it fits the unsigned target.
    let cb = 0.5 - 0.168736 * c.r - 0.331264 * c.g + 0.5 * c.b;
    let cr = 0.5 + 0.5 * c.r - 0.418688 * c.g - 0.081312 * c.b;
    return vec4<f32>(cb, cr, 0.0, 1.0);
}
"#,
        ));
        for _ in 0..depth {
            passes.push(
                GpuPass::new(
                    "color_norm_down",
                    r#"
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let dim = vec2<i32>(textureDimensions(tex));
    let o = vec2<i32>(in.pos.xy) * 2;
    let c00 = textureLoad(tex, min(o, dim - 1), 0);
    let c10 = textureLoad(tex, min(o + vec2<i32>(1, 0), dim - 1), 0);
    let c01 = textureLoad(tex, min(o + vec2<i32>(0, 1), dim - 1), 0);
    let c11 = textureLoad(tex, min(o + vec2<i32>(1, 1), dim - 1), 0);
    return (c00 + c10 + c01 + c11) * 0.25;
}
"#,
                )
                .with_out_size(OutSize::Half),
            );
        }
        passes.push(
            GpuPass::new(
                "color_norm_apply",
                r#"
struct P { v: array<vec4<f32>, 1> };
@group(0) @binding(2) var<uniform> p: P;

fn ycbcr_to_rgb(y: f32, cb: f32, cr: f32) -> vec3<f32> {
    let r = y + 1.402 * (cr - 0.5);
    let g = y - 0.344136 * (cb - 0.5) - 0.714136 * (cr - 0.5);
    let b = y + 1.772 * (cb - 0.5);
    return vec3<f32>(r, g, b);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let amount = p.v[0].x;
    // Smoothly upscaled coarse chroma at this pixel.
    let local = textureSampleLevel(tex, samp, in.uv, 0.0).rg;
    // Global mean chroma = average of the small level.
    let sdim = vec2<i32>(textureDimensions(tex));
    var sum = vec2<f32>(0.0);
    for (var y = 0; y < sdim.y; y = y + 1) {
        for (var x = 0; x < sdim.x; x = x + 1) {
            sum = sum + textureLoad(tex, vec2<i32>(x, y), 0).rg;
        }
    }
    let mean = sum / f32(sdim.x * sdim.y);
    let c = textureLoad(src_tex, vec2<i32>(in.pos.xy), 0);
    let yy = dot(c.rgb, vec3<f32>(0.299, 0.587, 0.114));
    let cb = 0.5 - 0.168736 * c.r - 0.331264 * c.g + 0.5 * c.b;
    let cr = 0.5 + 0.5 * c.r - 0.418688 * c.g - 0.081312 * c.b;
    // Subtract the low-frequency cast's deviation from the global mean, keeping
    // the pixel's own high-frequency chroma detail.
    let ncb = cb - amount * (local.r - mean.r);
    let ncr = cr - amount * (local.g - mean.g);
    let rgb = clamp(ycbcr_to_rgb(yy, ncb, ncr), vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(rgb, c.a);
}
"#,
            )
            .with_uniforms(&crate::gpu::uniforms(&[self.amount]))
            .with_out_size(OutSize::Source),
        );
        passes
    }
}
