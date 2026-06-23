use eframe::egui;

use crate::modifier::Modifier;
use crate::widgets;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct BrightnessContrast {
    brightness: f32,
    contrast: f32,
}

impl Modifier for BrightnessContrast {
    crate::modifier_serde!("brightness_contrast");

    fn name(&self) -> &'static str {
        "Brightness / Contrast"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        changed |= widgets::slider(ui, "Brightness", &mut self.brightness, -1.0..=1.0);
        changed |= widgets::slider(ui, "Contrast", &mut self.contrast, -1.0..=1.0);
        changed
    }

    fn gpu_pass(&self) -> Option<crate::gpu::GpuPass> {
        let factor = 1.0 + self.contrast;
        Some(
            crate::gpu::GpuPass::new(
                "brightness_contrast",
                r#"
struct P { v: array<vec4<f32>, 1> };
@group(0) @binding(2) var<uniform> p: P;
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureLoad(tex, vec2<i32>(in.pos.xy), 0);
    let brightness = p.v[0].x;
    let factor = p.v[0].y;
    let rgb = (c.rgb + vec3<f32>(brightness - 0.5)) * factor + vec3<f32>(0.5);
    return vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), c.a);
}
"#,
            )
            .with_uniforms(&crate::gpu::uniforms(&[self.brightness, factor])),
        )
    }
}
