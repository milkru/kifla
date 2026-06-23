use eframe::egui;

use crate::modifier::Modifier;
use crate::widgets;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct ShadowsHighlights {
    shadows: f32,
    highlights: f32,
}

impl Modifier for ShadowsHighlights {
    crate::modifier_serde!("shadows_highlights");

    fn name(&self) -> &'static str {
        "Shadows / Highlights"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        changed |= widgets::slider(ui, "Shadows", &mut self.shadows, -1.0..=1.0);
        changed |= widgets::slider(ui, "Highlights", &mut self.highlights, -1.0..=1.0);
        changed
    }

    fn gpu_pass(&self) -> Option<crate::gpu::GpuPass> {
        Some(
            crate::gpu::GpuPass::new(
                "shadows_highlights",
                r#"
struct P { v: array<vec4<f32>, 1> };
@group(0) @binding(2) var<uniform> p: P;
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureLoad(tex, vec2<i32>(in.pos.xy), 0);
    let shadows = p.v[0].x;
    let highlights = p.v[0].y;
    let lum = dot(c.rgb, vec3<f32>(0.299, 0.587, 0.114));
    let shadow_mask = (1.0 - lum) * (1.0 - lum);
    let highlight_mask = lum * lum;
    // Positive shadows lift toward white, negative deepen toward black;
    // positive highlights pull toward black, negative lift toward white.
    var rgb = c.rgb;
    let shadow_target = select(rgb, vec3<f32>(1.0) - rgb, shadows >= 0.0);
    rgb = rgb + shadows * shadow_mask * shadow_target;
    let highlight_target = select(vec3<f32>(1.0) - rgb, rgb, highlights >= 0.0);
    rgb = rgb - highlights * highlight_mask * highlight_target;
    return vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), c.a);
}
"#,
            )
            .with_uniforms(&crate::gpu::uniforms(&[self.shadows, self.highlights])),
        )
    }
}
