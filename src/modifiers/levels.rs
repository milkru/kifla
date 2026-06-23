use eframe::egui;

use crate::modifier::Modifier;
use crate::widgets;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Levels {
    in_black: f32,
    in_white: f32,
    gamma: f32,
    out_black: f32,
    out_white: f32,
}

impl Default for Levels {
    fn default() -> Self {
        Self {
            in_black: 0.0,
            in_white: 1.0,
            gamma: 1.0,
            out_black: 0.0,
            out_white: 1.0,
        }
    }
}

impl Modifier for Levels {
    crate::modifier_serde!("levels");

    fn name(&self) -> &'static str {
        "Levels"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        changed |= widgets::slider(ui, "Input black", &mut self.in_black, 0.0..=1.0);
        changed |= widgets::slider(ui, "Input white", &mut self.in_white, 0.0..=1.0);
        changed |= widgets::slider(ui, "Gamma", &mut self.gamma, 0.1..=5.0);
        ui.separator();
        changed |= widgets::slider(ui, "Output black", &mut self.out_black, 0.0..=1.0);
        changed |= widgets::slider(ui, "Output white", &mut self.out_white, 0.0..=1.0);
        changed
    }

    fn gpu_pass(&self) -> Option<crate::gpu::GpuPass> {
        let denom = (self.in_white - self.in_black).max(1e-4);
        let inv_gamma = 1.0 / self.gamma;
        Some(
            crate::gpu::GpuPass::new(
                "levels",
                r#"
struct P { v: array<vec4<f32>, 2> };
@group(0) @binding(2) var<uniform> p: P;
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureLoad(tex, vec2<i32>(in.pos.xy), 0);
    let in_black = p.v[0].x;
    let denom = p.v[0].y;
    let inv_gamma = p.v[0].z;
    let out_black = p.v[0].w;
    let out_white = p.v[1].x;
    let n = pow(clamp((c.rgb - vec3<f32>(in_black)) / denom, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(inv_gamma));
    let rgb = vec3<f32>(out_black) + n * (out_white - out_black);
    return vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), c.a);
}
"#,
            )
            .with_uniforms(&crate::gpu::uniforms(&[
                self.in_black,
                denom,
                inv_gamma,
                self.out_black,
                self.out_white,
            ])),
        )
    }
}
