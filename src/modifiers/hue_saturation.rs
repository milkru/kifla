use eframe::egui;

use crate::modifier::Modifier;
use crate::widgets;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct HueSaturation {
    hue: f32,
    saturation: f32,
    lightness: f32,
}

impl Modifier for HueSaturation {
    crate::modifier_serde!("hue_saturation");

    fn name(&self) -> &'static str {
        "Hue / Saturation"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        changed |= widgets::slider(ui, "Hue", &mut self.hue, -180.0..=180.0);
        changed |= widgets::slider(ui, "Saturation", &mut self.saturation, -1.0..=1.0);
        changed |= widgets::slider(ui, "Lightness", &mut self.lightness, -1.0..=1.0);
        changed
    }

    fn gpu_pass(&self) -> Option<crate::gpu::GpuPass> {
        let hue_shift = self.hue / 360.0;
        Some(
            crate::gpu::GpuPass::new(
                "hue_saturation",
                r#"
struct P { v: array<vec4<f32>, 1> };
@group(0) @binding(2) var<uniform> p: P;
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureLoad(tex, vec2<i32>(in.pos.xy), 0);
    let hue_shift = p.v[0].x;
    let sat = p.v[0].y;
    let light = p.v[0].z;
    var hsl = rgb_to_hsl(c.rgb);
    let hh = hsl.x + hue_shift;
    hsl.x = hh - floor(hh);
    hsl.y = clamp(hsl.y * (1.0 + sat), 0.0, 1.0);
    hsl.z = clamp(hsl.z + light, 0.0, 1.0);
    let rgb = clamp(hsl_to_rgb(hsl), vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(rgb, c.a);
}
"#,
            )
            .with_uniforms(&crate::gpu::uniforms(&[
                hue_shift,
                self.saturation,
                self.lightness,
            ])),
        )
    }
}
