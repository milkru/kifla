use eframe::egui;

use crate::modifier::Modifier;
use crate::widgets;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Repeat {
    x: f32,
    y: f32,
}

impl Default for Repeat {
    fn default() -> Self {
        Self { x: 1.0, y: 1.0 }
    }
}

impl Modifier for Repeat {
    crate::modifier_serde!("repeat");

    fn name(&self) -> &'static str {
        "Repeat"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        changed |= widgets::slider(ui, "X", &mut self.x, 1.0..=32.0);
        changed |= widgets::slider(ui, "Y", &mut self.y, 1.0..=32.0);
        changed
    }

    fn gpu_pass(&self) -> Option<crate::gpu::GpuPass> {
        Some(
            crate::gpu::GpuPass::new(
                "repeat",
                r#"
struct P { v: array<vec4<f32>, 1> };
@group(0) @binding(2) var<uniform> p: P;
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let dim = vec2<i32>(textureDimensions(tex));
    let rx = clamp(p.v[0].x, 1.0, 32.0);
    let ry = clamp(p.v[0].y, 1.0, 32.0);
    let sxn = u32(clamp(ceil(rx), 1.0, 4.0));
    let syn = u32(clamp(ceil(ry), 1.0, 4.0));
    let oc = floor(in.pos.xy);
    // Accumulate in 0..255 integer space and floor the mean, mirroring the CPU
    // path's integer averaging (so flat regions match exactly, no rounding drift).
    var acc = vec4<f32>(0.0);
    var count = 0.0;
    for (var j = 0u; j < syn; j = j + 1u) {
        let fy = oc.y * ry + f32(j) * ry / f32(syn);
        let sy = ((i32(floor(fy)) % dim.y) + dim.y) % dim.y;
        for (var i = 0u; i < sxn; i = i + 1u) {
            let fx = oc.x * rx + f32(i) * rx / f32(sxn);
            let sx = ((i32(floor(fx)) % dim.x) + dim.x) % dim.x;
            acc = acc + round(textureLoad(tex, vec2<i32>(sx, sy), 0) * 255.0);
            count = count + 1.0;
        }
    }
    return floor(acc / count) / 255.0;
}
"#,
            )
            .with_uniforms(&crate::gpu::uniforms(&[self.x, self.y])),
        )
    }
}
