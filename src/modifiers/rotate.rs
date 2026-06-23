use eframe::egui;
use image::imageops;

use crate::modifier::Modifier;
use crate::pixel::remap_wrap;
use crate::widgets;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Rotate {
    angle: f32,
}

impl Modifier for Rotate {
    crate::modifier_serde!("rotate");

    fn name(&self) -> &'static str {
        "Rotate"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        widgets::slider(ui, "Angle", &mut self.angle, -45.0..=45.0)
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        if self.angle == 0.0 {
            return;
        }
        let (cx, cy) = (image.width() as f32 * 0.5, image.height() as f32 * 0.5);
        let (sin, cos) = self.angle.to_radians().sin_cos();
        remap_wrap(image, |ox, oy| {
            let (dx, dy) = (ox - cx, oy - cy);
            (cx + dx * cos + dy * sin, cy - dx * sin + dy * cos)
        });
    }

    fn gpu_pass(&self) -> Option<crate::gpu::GpuPass> {
        let (sin, cos) = self.angle.to_radians().sin_cos();
        Some(
            crate::gpu::GpuPass::new(
                "rotate",
                r#"
struct P { v: array<vec4<f32>, 1> };
@group(0) @binding(2) var<uniform> p: P;
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let dim = vec2<f32>(textureDimensions(tex));
    let c = dim * 0.5;
    let sn = p.v[0].x;
    let cs = p.v[0].y;
    let o = floor(in.pos.xy);
    let d = o - c;
    let src = vec2<f32>(c.x + d.x * cs + d.y * sn, c.y - d.x * sn + d.y * cs);
    return sample_wrap(src);
}
"#,
            )
            .with_uniforms(&crate::gpu::uniforms(&[sin, cos])),
        )
    }
}

pub struct Rotate90Cw;

impl Modifier for Rotate90Cw {
    crate::modifier_id!("rotate_90_cw");

    fn name(&self) -> &'static str {
        "Rotate 90° CW"
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        *image = imageops::rotate90(image);
    }

    fn gpu_pass(&self) -> Option<crate::gpu::GpuPass> {
        // 90° CW: output is (H x W); output(x, y) = input(y, H_src - 1 - x).
        // textureDimensions(tex) is the *input* size (W x H).
        Some(crate::gpu::GpuPass::new(
            "rotate_90_cw",
            r#"
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let dim = vec2<i32>(textureDimensions(tex));
    let o = vec2<i32>(in.pos.xy);
    return textureLoad(tex, vec2<i32>(o.y, dim.y - 1 - o.x), 0);
}
"#,
        ).with_out_size(crate::gpu::OutSize::Swap))
    }
}

pub struct Rotate90Ccw;

impl Modifier for Rotate90Ccw {
    crate::modifier_id!("rotate_90_ccw");

    fn name(&self) -> &'static str {
        "Rotate 90° CCW"
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        *image = imageops::rotate270(image);
    }

    fn gpu_pass(&self) -> Option<crate::gpu::GpuPass> {
        // 90° CCW: output is (H x W); output(x, y) = input(W_src - 1 - y, x).
        Some(
            crate::gpu::GpuPass::new(
                "rotate_90_ccw",
                r#"
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let dim = vec2<i32>(textureDimensions(tex));
    let o = vec2<i32>(in.pos.xy);
    return textureLoad(tex, vec2<i32>(dim.x - 1 - o.y, o.x), 0);
}
"#,
            )
            .with_out_size(crate::gpu::OutSize::Swap),
        )
    }
}
