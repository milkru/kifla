use eframe::egui;

use crate::edit::Edit;
use crate::pixel::remap_wrap;
use crate::widgets;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Skew {
    horizontal: f32,
    vertical: f32,
}

impl Edit for Skew {
    crate::edit_serde!("skew");

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
}
