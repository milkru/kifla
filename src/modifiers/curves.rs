use eframe::egui;

use crate::modifier::Modifier;
use crate::widgets;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Curves {
    points: Vec<egui::Pos2>,
}

impl Default for Curves {
    fn default() -> Self {
        Self {
            points: vec![
                egui::pos2(0.0, 0.0),
                egui::pos2(0.5, 0.5),
                egui::pos2(1.0, 1.0),
            ],
        }
    }
}

impl Modifier for Curves {
    crate::modifier_serde!("curves");

    fn name(&self) -> &'static str {
        "Curves"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        widgets::curve_editor(ui, &mut self.points)
    }

    fn gpu_pass(&self) -> Option<crate::gpu::GpuPass> {
        // Pass the 256-entry LUT (normalized) as 64 vec4s and look it up by the
        // 8-bit channel value, matching the CPU table exactly.
        let lut = build_lut(&self.points);
        let table: Vec<f32> = lut.iter().map(|&v| v as f32 / 255.0).collect();
        Some(
            crate::gpu::GpuPass::new(
                "curves",
                r#"
struct P { lut: array<vec4<f32>, 64> };
@group(0) @binding(2) var<uniform> p: P;
fn lookup(value: f32) -> f32 {
    let i = i32(round(clamp(value, 0.0, 1.0) * 255.0));
    return p.lut[i >> 2u][u32(i) & 3u];
}
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureLoad(tex, vec2<i32>(in.pos.xy), 0);
    return vec4<f32>(lookup(c.r), lookup(c.g), lookup(c.b), c.a);
}
"#,
            )
            .with_uniforms(&crate::gpu::uniforms(&table)),
        )
    }
}

fn build_lut(points: &[egui::Pos2]) -> [u8; 256] {
    let mut lut = [0u8; 256];
    for (i, slot) in lut.iter_mut().enumerate() {
        let y = widgets::curve_value(points, i as f32 / 255.0).clamp(0.0, 1.0);
        *slot = (y * 255.0).round() as u8;
    }
    lut
}
