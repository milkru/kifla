use eframe::egui;

use crate::modifier::Modifier;
use crate::pixel::remap_wrap;
use crate::widgets;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Skew {
    horizontal: f32,
    vertical: f32,
}

impl Modifier for Skew {
    crate::modifier_serde!("skew");

    fn name(&self) -> &'static str {
        "Skew"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        changed |= widgets::slider(ui, "Horizontal", &mut self.horizontal, -45.0..=45.0);
        changed |= widgets::slider(ui, "Vertical", &mut self.vertical, -45.0..=45.0);
        changed
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        if self.horizontal == 0.0 && self.vertical == 0.0 {
            return;
        }
        let (cx, cy) = (image.width() as f32 * 0.5, image.height() as f32 * 0.5);
        let kx = self.horizontal.to_radians().tan();
        let ky = self.vertical.to_radians().tan();
        remap_wrap(image, |ox, oy| {
            let sy = oy - ky * (ox - cx);
            let sx = ox - kx * (sy - cy);
            (sx, sy)
        });
    }

    fn gpu_pass(&self) -> Option<crate::gpu::GpuPass> {
        let kx = self.horizontal.to_radians().tan();
        let ky = self.vertical.to_radians().tan();
        Some(
            crate::gpu::GpuPass::new(
                "skew",
                r#"
struct P { v: array<vec4<f32>, 1> };
@group(0) @binding(2) var<uniform> p: P;
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let dim = vec2<f32>(textureDimensions(tex));
    let c = dim * 0.5;
    let kx = p.v[0].x;
    let ky = p.v[0].y;
    let o = floor(in.pos.xy);
    let sy = o.y - ky * (o.x - c.x);
    let sx = o.x - kx * (sy - c.y);
    return sample_wrap(vec2<f32>(sx, sy));
}
"#,
            )
            .with_uniforms(&crate::gpu::uniforms(&[kx, ky])),
        )
    }
}
