use eframe::egui;

use crate::color;
use crate::modifier::Modifier;
use crate::pixel::{par_pixels, to_u8};
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

    fn apply(&self, image: &mut image::RgbaImage) {
        let hue_shift = self.hue / 360.0;
        par_pixels(image, |px| {
            let r = px[0] as f32 / 255.0;
            let g = px[1] as f32 / 255.0;
            let b = px[2] as f32 / 255.0;

            let (mut h, mut s, mut l) = color::rgb_to_hsl(r, g, b);
            h = (h + hue_shift).rem_euclid(1.0);
            s = (s * (1.0 + self.saturation)).clamp(0.0, 1.0);
            l = (l + self.lightness).clamp(0.0, 1.0);

            let (nr, ng, nb) = color::hsl_to_rgb(h, s, l);
            px[0] = to_u8(nr);
            px[1] = to_u8(ng);
            px[2] = to_u8(nb);
        });
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
