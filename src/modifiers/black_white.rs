use eframe::egui;

use crate::modifier::Modifier;
use crate::widgets;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BlackWhite {
    red: f32,
    green: f32,
    blue: f32,
    amount: f32,
}

impl Default for BlackWhite {
    fn default() -> Self {
        Self {
            red: 0.3,
            green: 0.59,
            blue: 0.11,
            amount: 0.0,
        }
    }
}

impl Modifier for BlackWhite {
    crate::modifier_serde!("black_white");

    fn name(&self) -> &'static str {
        "Black & White"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        changed |= widgets::slider(ui, "Red", &mut self.red, -0.5..=1.5);
        changed |= widgets::slider(ui, "Green", &mut self.green, -0.5..=1.5);
        changed |= widgets::slider(ui, "Blue", &mut self.blue, -0.5..=1.5);
        ui.separator();
        changed |= widgets::slider(ui, "Amount", &mut self.amount, 0.0..=1.0);
        changed
    }

    fn gpu_pass(&self) -> Option<crate::gpu::GpuPass> {
        Some(
            crate::gpu::GpuPass::new(
                "black_white",
                r#"
struct P { v: array<vec4<f32>, 1> };
@group(0) @binding(2) var<uniform> p: P;
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureLoad(tex, vec2<i32>(in.pos.xy), 0);
    let weights = p.v[0].xyz;
    let amount = p.v[0].w;
    let gray = clamp(dot(c.rgb, weights), 0.0, 1.0);
    let rgb = c.rgb * (1.0 - amount) + vec3<f32>(gray) * amount;
    return vec4<f32>(rgb, c.a);
}
"#,
            )
            .with_uniforms(&crate::gpu::uniforms(&[
                self.red,
                self.green,
                self.blue,
                self.amount,
            ])),
        )
    }
}
