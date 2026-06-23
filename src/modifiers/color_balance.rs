use eframe::egui;

use crate::color;
use crate::modifier::Modifier;
use crate::pixel::{par_pixels, to_u8};
use crate::widgets;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct ColorBalance {
    shadows: [f32; 3],
    midtones: [f32; 3],
    highlights: [f32; 3],
}

fn range_ui(ui: &mut egui::Ui, title: &str, amounts: &mut [f32; 3]) -> bool {
    ui.label(title);
    let mut changed = false;
    changed |= widgets::slider(ui, "Cyan-Red", &mut amounts[0], -1.0..=1.0);
    changed |= widgets::slider(ui, "Magenta-Green", &mut amounts[1], -1.0..=1.0);
    changed |= widgets::slider(ui, "Yellow-Blue", &mut amounts[2], -1.0..=1.0);
    changed
}

impl Modifier for ColorBalance {
    crate::modifier_serde!("color_balance");

    fn name(&self) -> &'static str {
        "Color Balance"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        changed |= range_ui(ui, "Shadows", &mut self.shadows);
        ui.separator();
        changed |= range_ui(ui, "Midtones", &mut self.midtones);
        ui.separator();
        changed |= range_ui(ui, "Highlights", &mut self.highlights);
        changed
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        const STRENGTH: f32 = 0.5;
        par_pixels(image, |px| {
            let rgb = [
                px[0] as f32 / 255.0,
                px[1] as f32 / 255.0,
                px[2] as f32 / 255.0,
            ];
            let lum = color::luma(rgb[0], rgb[1], rgb[2]);
            let shadow = (1.0 - 2.0 * lum).max(0.0);
            let highlight = (2.0 * lum - 1.0).max(0.0);
            let midtone = (1.0 - shadow - highlight).max(0.0);

            for c in 0..3 {
                let shift = STRENGTH
                    * (self.shadows[c] * shadow
                        + self.midtones[c] * midtone
                        + self.highlights[c] * highlight);
                px[c] = to_u8(rgb[c] + shift);
            }
        });
    }

    fn gpu_pass(&self) -> Option<crate::gpu::GpuPass> {
        Some(
            crate::gpu::GpuPass::new(
                "color_balance",
                r#"
struct P { v: array<vec4<f32>, 3> };
@group(0) @binding(2) var<uniform> p: P;
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureLoad(tex, vec2<i32>(in.pos.xy), 0);
    let lum = dot(c.rgb, vec3<f32>(0.299, 0.587, 0.114));
    let shadow = max(1.0 - 2.0 * lum, 0.0);
    let highlight = max(2.0 * lum - 1.0, 0.0);
    let midtone = max(1.0 - shadow - highlight, 0.0);
    let shift = 0.5 * (p.v[0].xyz * shadow + p.v[1].xyz * midtone + p.v[2].xyz * highlight);
    let rgb = clamp(c.rgb + shift, vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(rgb, c.a);
}
"#,
            )
            .with_uniforms(&crate::gpu::uniforms(&[
                self.shadows[0], self.shadows[1], self.shadows[2], 0.0,
                self.midtones[0], self.midtones[1], self.midtones[2], 0.0,
                self.highlights[0], self.highlights[1], self.highlights[2], 0.0,
            ])),
        )
    }
}
