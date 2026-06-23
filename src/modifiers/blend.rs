use eframe::egui;

use crate::modifier::Modifier;
use crate::widgets;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Blend {
    falloff: f32,
    overlap_x: f32,
    overlap_y: f32,
}

impl Default for Blend {
    fn default() -> Self {
        Self {
            falloff: 0.5,
            overlap_x: 0.0,
            overlap_y: 0.0,
        }
    }
}

impl Modifier for Blend {
    crate::modifier_serde!("blend");

    fn name(&self) -> &'static str {
        "Blend"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        if !ui.is_enabled() {
            ui.label(format!("Falloff: {:.2}", self.falloff));
            ui.label(format!("Overlap X: {:.2}", self.overlap_x));
            ui.label(format!("Overlap Y: {:.2}", self.overlap_y));
            return false;
        }
        let mut changed = false;
        changed |= widgets::slider(ui, "Edge Falloff", &mut self.falloff, 0.01..=1.0);
        changed |= widgets::slider(ui, "Overlap X", &mut self.overlap_x, 0.0..=1.0);
        changed |= widgets::slider(ui, "Overlap Y", &mut self.overlap_y, 0.0..=1.0);
        changed
    }

    fn gpu_pass(&self) -> Option<crate::gpu::GpuPass> {
        Some(
            crate::gpu::GpuPass::new(
                "blend",
                r#"
struct P { v: array<vec4<f32>, 1> };
@group(0) @binding(2) var<uniform> p: P;
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let dim = vec2<i32>(textureDimensions(tex));
    let overlap_x = p.v[0].x;
    let overlap_y = p.v[0].y;
    let pw = p.v[0].z;
    let coord = vec2<i32>(in.pos.xy);
    let band_x = overlap_x * 0.5 * f32(dim.x);
    let band_y = overlap_y * 0.5 * f32(dim.y);
    let ow = dim.x / 2;
    let oh = dim.y / 2;
    let xs = (coord.x + ow) % dim.x;
    let ys = (coord.y + oh) % dim.y;

    let dye = f32(min(coord.y, dim.y - 1 - coord.y));
    var wy = 0.0;
    if (band_y > 0.0 && dye < band_y) { wy = clamp(pow(1.0 - dye / band_y, pw), 0.0, 1.0); }
    let dxe = f32(min(coord.x, dim.x - 1 - coord.x));
    var wx = 0.0;
    if (band_x > 0.0 && dxe < band_x) { wx = clamp(pow(1.0 - dxe / band_x, pw), 0.0, 1.0); }

    let orig = textureLoad(tex, coord, 0);
    let s_x = textureLoad(tex, vec2<i32>(xs, coord.y), 0);
    let s_y = textureLoad(tex, vec2<i32>(coord.x, ys), 0);
    let s_xy = textureLoad(tex, vec2<i32>(xs, ys), 0);
    let healed_x = mix(orig, s_x, wx);
    let healed_y = mix(s_y, s_xy, wx);
    return mix(healed_x, healed_y, wy);
}
"#,
            )
            .with_uniforms(&crate::gpu::uniforms(&[
                self.overlap_x,
                self.overlap_y,
                1.0 / self.falloff.clamp(0.01, 1.0),
            ])),
        )
    }
}
