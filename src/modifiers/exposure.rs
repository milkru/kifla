use eframe::egui;

use crate::modifier::Modifier;
use crate::widgets;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Exposure {
    exposure: f32,
    offset: f32,
    gamma: f32,
}

impl Default for Exposure {
    fn default() -> Self {
        Self {
            exposure: 0.0,
            offset: 0.0,
            gamma: 1.0,
        }
    }
}

impl Modifier for Exposure {
    crate::modifier_serde!("exposure");

    fn name(&self) -> &'static str {
        "Exposure"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        changed |= widgets::slider(ui, "Exposure", &mut self.exposure, -3.0..=3.0);
        changed |= widgets::slider(ui, "Offset", &mut self.offset, -0.5..=0.5);
        changed |= widgets::slider(ui, "Gamma", &mut self.gamma, 0.1..=5.0);
        changed
    }

    fn gpu_pass(&self) -> Option<crate::gpu::GpuPass> {
        let mult = 2f32.powf(self.exposure);
        let inv_gamma = 1.0 / self.gamma;
        Some(
            crate::gpu::GpuPass::new(
                "exposure",
                r#"
struct P { v: array<vec4<f32>, 1> };
@group(0) @binding(2) var<uniform> p: P;
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureLoad(tex, vec2<i32>(in.pos.xy), 0);
    let mult = p.v[0].x;
    let offset = p.v[0].y;
    let inv_gamma = p.v[0].z;
    let rgb = pow(max(c.rgb * mult + vec3<f32>(offset), vec3<f32>(0.0)), vec3<f32>(inv_gamma));
    return vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), c.a);
}
"#,
            )
            .with_uniforms(&crate::gpu::uniforms(&[mult, self.offset, inv_gamma])),
        )
    }
}
