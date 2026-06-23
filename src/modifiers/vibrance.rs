use eframe::egui;

use crate::modifier::Modifier;
use crate::widgets;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Vibrance {
    vibrance: f32,
    saturation: f32,
}

impl Modifier for Vibrance {
    crate::modifier_serde!("vibrance");

    fn name(&self) -> &'static str {
        "Vibrance"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        changed |= widgets::slider(ui, "Vibrance", &mut self.vibrance, -1.0..=1.0);
        changed |= widgets::slider(ui, "Saturation", &mut self.saturation, -1.0..=1.0);
        changed
    }

    fn gpu_pass(&self) -> Option<crate::gpu::GpuPass> {
        Some(
            crate::gpu::GpuPass::new(
                "vibrance",
                r#"
struct P { v: array<vec4<f32>, 1> };
@group(0) @binding(2) var<uniform> p: P;
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureLoad(tex, vec2<i32>(in.pos.xy), 0);
    let vib = p.v[0].x;
    let sat = p.v[0].y;
    var hsl = rgb_to_hsl(c.rgb);
    var s = hsl.y * (1.0 + sat);
    s += vib * (1.0 - s);
    hsl.y = clamp(s, 0.0, 1.0);
    let rgb = clamp(hsl_to_rgb(hsl), vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(rgb, c.a);
}
"#,
            )
            .with_uniforms(&crate::gpu::uniforms(&[self.vibrance, self.saturation])),
        )
    }
}
