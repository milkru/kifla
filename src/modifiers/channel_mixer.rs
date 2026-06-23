use eframe::egui;

use crate::modifier::Modifier;
use crate::widgets;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ChannelMixer {
    red: [f32; 3],
    green: [f32; 3],
    blue: [f32; 3],
}

impl Default for ChannelMixer {
    fn default() -> Self {
        Self {
            red: [1.0, 0.0, 0.0],
            green: [0.0, 1.0, 0.0],
            blue: [0.0, 0.0, 1.0],
        }
    }
}

fn output_ui(ui: &mut egui::Ui, title: &str, weights: &mut [f32; 3]) -> bool {
    ui.label(title);
    let mut changed = false;
    changed |= widgets::slider(ui, "Red", &mut weights[0], -2.0..=2.0);
    changed |= widgets::slider(ui, "Green", &mut weights[1], -2.0..=2.0);
    changed |= widgets::slider(ui, "Blue", &mut weights[2], -2.0..=2.0);
    changed
}

impl Modifier for ChannelMixer {
    crate::modifier_serde!("channel_mixer");

    fn name(&self) -> &'static str {
        "Channel Mixer"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        changed |= output_ui(ui, "Output Red", &mut self.red);
        ui.separator();
        changed |= output_ui(ui, "Output Green", &mut self.green);
        ui.separator();
        changed |= output_ui(ui, "Output Blue", &mut self.blue);
        changed
    }

    fn gpu_pass(&self) -> Option<crate::gpu::GpuPass> {
        Some(
            crate::gpu::GpuPass::new(
                "channel_mixer",
                r#"
struct P { v: array<vec4<f32>, 3> };
@group(0) @binding(2) var<uniform> p: P;
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureLoad(tex, vec2<i32>(in.pos.xy), 0);
    let nr = dot(p.v[0].xyz, c.rgb);
    let ng = dot(p.v[1].xyz, c.rgb);
    let nb = dot(p.v[2].xyz, c.rgb);
    let rgb = clamp(vec3<f32>(nr, ng, nb), vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(rgb, c.a);
}
"#,
            )
            .with_uniforms(&crate::gpu::uniforms(&[
                self.red[0], self.red[1], self.red[2], 0.0,
                self.green[0], self.green[1], self.green[2], 0.0,
                self.blue[0], self.blue[1], self.blue[2], 0.0,
            ])),
        )
    }
}
