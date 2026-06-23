use eframe::egui;
use image::imageops;

use crate::modifier::Modifier;
use crate::widgets;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Crop {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    src_width: u32,
    src_height: u32,
}

impl Modifier for Crop {
    crate::modifier_serde!("crop");

    fn name(&self) -> &'static str {
        "Crop"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn on_added(&mut self, width: u32, height: u32) {
        self.x = 0;
        self.y = 0;
        self.width = width;
        self.height = height;
        self.src_width = width;
        self.src_height = height;
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        if !ui.is_enabled() {
            ui.label(format!("X: {}", self.x));
            ui.label(format!("Y: {}", self.y));
            ui.label(format!("Width: {}", self.width));
            ui.label(format!("Height: {}", self.height));
            return false;
        }

        let max_w = self.src_width.max(1);
        let max_h = self.src_height.max(1);
        self.x = self.x.min(max_w - 1);
        self.y = self.y.min(max_h - 1);
        self.width = self.width.clamp(1, max_w - self.x);
        self.height = self.height.clamp(1, max_h - self.y);

        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("X");
            changed |= widgets::drag_value(ui, &mut self.x, 0..=max_w - 1);
        });
        ui.horizontal(|ui| {
            ui.label("Y");
            changed |= widgets::drag_value(ui, &mut self.y, 0..=max_h - 1);
        });
        ui.horizontal(|ui| {
            ui.label("Width");
            changed |= widgets::drag_value(ui, &mut self.width, 1..=max_w - self.x);
        });
        ui.horizontal(|ui| {
            ui.label("Height");
            changed |= widgets::drag_value(ui, &mut self.height, 1..=max_h - self.y);
        });
        changed
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        let (iw, ih) = (image.width(), image.height());
        if iw == 0 || ih == 0 {
            return;
        }
        let x = self.x.min(iw - 1);
        let y = self.y.min(ih - 1);
        let width = self.width.clamp(1, iw - x);
        let height = self.height.clamp(1, ih - y);
        if x == 0 && y == 0 && width == iw && height == ih {
            return;
        }
        *image = imageops::crop_imm(image, x, y, width, height).to_image();
    }

    fn gpu_pass(&self) -> Option<crate::gpu::GpuPass> {
        // Output is the crop rect; each output pixel reads the input offset by
        // (x, y). The region fits the input by construction (clamped in the UI
        // against the source size), so no out-of-bounds reads occur.
        Some(
            crate::gpu::GpuPass::new(
                "crop",
                r#"
struct P { v: array<vec4<f32>, 1> };
@group(0) @binding(2) var<uniform> p: P;
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let off = vec2<i32>(i32(p.v[0].x), i32(p.v[0].y));
    return textureLoad(tex, vec2<i32>(in.pos.xy) + off, 0);
}
"#,
            )
            .with_uniforms(&crate::gpu::uniforms(&[self.x as f32, self.y as f32]))
            .with_out_size(crate::gpu::OutSize::Fixed(self.width.max(1), self.height.max(1))),
        )
    }
}
