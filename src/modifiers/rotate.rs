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
}
