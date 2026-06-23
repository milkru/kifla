use eframe::egui;

use crate::modifier::Modifier;
use crate::widgets;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Lighting {
    amount: f32,
    #[serde(default)]
    width: u32,
    #[serde(default)]
    height: u32,
}

impl Modifier for Lighting {
    crate::modifier_serde!("lighting");

    fn name(&self) -> &'static str {
        "Lighting Normalization"
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

        // Luminance field, then a downsample pyramid approximating the heavy
        // blur (illumination estimate), then a final pass that divides the
        // source by the upscaled illumination and restores per-pixel detail.
        // `width`/`height` are set when the modifier is added (and persisted in
        // the stack); fall back to a reasonable size if somehow unknown.
        let maxd = self.width.max(self.height).max(1) as f32;
        let maxd = if maxd <= 1.0 { 1024.0 } else { maxd };
        let depth = (maxd.log2().floor() as i32 - 2).clamp(1, 12) as usize;

        let mut passes = Vec::with_capacity(depth + 2);
        passes.push(GpuPass::new(
            "lighting_lum",
            r#"
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureLoad(tex, vec2<i32>(in.pos.xy), 0);
    let l = dot(c.rgb, vec3<f32>(0.299, 0.587, 0.114));
    return vec4<f32>(l, l, l, 1.0);
}
"#,
        ));
        for _ in 0..depth {
            passes.push(
                GpuPass::new(
                    "lighting_down",
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
                "lighting_apply",
                r#"
struct P { v: array<vec4<f32>, 1> };
@group(0) @binding(2) var<uniform> p: P;
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let amount = p.v[0].x;
    // Smoothly upscaled coarse illumination at this pixel.
    let illum = textureSampleLevel(tex, samp, in.uv, 0.0).r;
    // Global mean luminance = average of the small level.
    let sdim = vec2<i32>(textureDimensions(tex));
    var sum = 0.0;
    for (var y = 0; y < sdim.y; y = y + 1) {
        for (var x = 0; x < sdim.x; x = x + 1) {
            sum = sum + textureLoad(tex, vec2<i32>(x, y), 0).r;
        }
    }
    let mean = sum / f32(sdim.x * sdim.y);
    let c = textureLoad(src_tex, vec2<i32>(in.pos.xy), 0);
    var gain = clamp(mean / max(illum, 1e-5), 0.2, 5.0);
    gain = max(1.0 + (gain - 1.0) * amount, 0.0);
    let rgb = clamp(c.rgb * gain, vec3<f32>(0.0), vec3<f32>(1.0));
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
