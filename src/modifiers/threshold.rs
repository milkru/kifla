use eframe::egui;

use crate::color;
use crate::modifier::Modifier;
use crate::pixel::par_pixels;
use crate::widgets;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Threshold {
    level: f32,
    amount: f32,
}

impl Default for Threshold {
    fn default() -> Self {
        Self {
            level: 0.5,
            amount: 0.0,
        }
    }
}

impl Modifier for Threshold {
    crate::modifier_serde!("threshold");

    fn name(&self) -> &'static str {
        "Threshold"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        changed |= widgets::slider(ui, "Threshold", &mut self.level, 0.0..=1.0);
        changed |= widgets::slider(ui, "Amount", &mut self.amount, 0.0..=1.0);
        changed
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        if self.amount <= 0.0 {
            return;
        }
        par_pixels(image, |px| {
            let lum = color::luma(px[0] as f32, px[1] as f32, px[2] as f32) / 255.0;
            let value = if lum >= self.level { 255.0 } else { 0.0 };
            for channel in &mut px[..3] {
                let blended = *channel as f32 * (1.0 - self.amount) + value * self.amount;
                *channel = blended.round() as u8;
            }
        });
    }

    fn gpu_pass(&self) -> Option<crate::gpu::GpuPass> {
        Some(
            crate::gpu::GpuPass::new(
                "threshold",
                r#"
struct P { v: array<vec4<f32>, 1> };
@group(0) @binding(2) var<uniform> p: P;
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureLoad(tex, vec2<i32>(in.pos.xy), 0);
    let level = p.v[0].x;
    let amount = p.v[0].y;
    let lum = dot(c.rgb, vec3<f32>(0.299, 0.587, 0.114));
    let value = select(0.0, 1.0, lum >= level);
    let rgb = c.rgb * (1.0 - amount) + vec3<f32>(value) * amount;
    return vec4<f32>(rgb, c.a);
}
"#,
            )
            .with_uniforms(&crate::gpu::uniforms(&[self.level, self.amount])),
        )
    }
}
