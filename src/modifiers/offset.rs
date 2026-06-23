use eframe::egui;

use crate::modifier::Modifier;
use crate::widgets;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Offset {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl Modifier for Offset {
    crate::modifier_serde!("offset");

    fn name(&self) -> &'static str {
        "Offset"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn on_added(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        if !ui.is_enabled() {
            ui.label(format!("X: {}", self.x));
            ui.label(format!("Y: {}", self.y));
            return false;
        }

        let bx = self.width.max(1) as i32;
        let by = self.height.max(1) as i32;
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("X");
            changed |= widgets::drag_value(ui, &mut self.x, -bx..=bx);
        });
        ui.horizontal(|ui| {
            ui.label("Y");
            changed |= widgets::drag_value(ui, &mut self.y, -by..=by);
        });
        changed
    }

    fn gpu_pass(&self) -> Option<crate::gpu::GpuPass> {
        Some(
            crate::gpu::GpuPass::new(
                "offset",
                r#"
struct P { v: array<vec4<f32>, 1> };
@group(0) @binding(2) var<uniform> p: P;
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let dim = vec2<i32>(textureDimensions(tex));
    let coord = vec2<i32>(in.pos.xy);
    let raw = vec2<i32>(i32(p.v[0].x), i32(p.v[0].y));
    let off = ((raw % dim) + dim) % dim;
    let src = ((coord - off) % dim + dim) % dim;
    return textureLoad(tex, src, 0);
}
"#,
            )
            .with_uniforms(&crate::gpu::uniforms(&[self.x as f32, self.y as f32])),
        )
    }
}
