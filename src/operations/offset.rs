use eframe::egui;
use image::RgbaImage;
use rayon::prelude::*;

use crate::operation::Operation;

fn offset_wrap(image: &RgbaImage, ox: i64, oy: i64) -> RgbaImage {
    let (w, h) = (image.width(), image.height());
    if w == 0 || h == 0 {
        return image.clone();
    }
    let oxm = ox.rem_euclid(w as i64) as u32;
    let oym = oy.rem_euclid(h as i64) as u32;
    if oxm == 0 && oym == 0 {
        return image.clone();
    }

    let mut out = RgbaImage::new(w, h);
    let row_len = w as usize * 4;
    let oxb = oxm as usize * 4;
    let cut = (w - oxm) as usize * 4;
    let src: &[u8] = image;
    let dst: &mut [u8] = &mut out;

    dst.par_chunks_mut(row_len)
        .enumerate()
        .for_each(|(y, row)| {
            let sy = ((y as u32 + h - oym) % h) as usize;
            let s = &src[sy * row_len..sy * row_len + row_len];
            row[0..oxb].copy_from_slice(&s[cut..row_len]);
            row[oxb..row_len].copy_from_slice(&s[0..cut]);
        });

    out
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Offset {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl Default for Offset {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }
}

impl Operation for Offset {
    crate::op_serde!("offset");

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
            let r = ui.add(egui::DragValue::new(&mut self.x).clamp_range(-bx..=bx));
            changed |= r.drag_released() || r.lost_focus();
        });
        ui.horizontal(|ui| {
            ui.label("Y");
            let r = ui.add(egui::DragValue::new(&mut self.y).clamp_range(-by..=by));
            changed |= r.drag_released() || r.lost_focus();
        });
        changed
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        if self.x == 0 && self.y == 0 {
            return;
        }
        *image = offset_wrap(image, self.x as i64, self.y as i64);
    }
}

pub struct OffsetHalfWidth;

impl Operation for OffsetHalfWidth {
    crate::op_id!("offset_half_width");

    fn name(&self) -> &'static str {
        "Offset Half Width"
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        *image = offset_wrap(image, (image.width() / 2) as i64, 0);
    }
}

pub struct OffsetHalfHeight;

impl Operation for OffsetHalfHeight {
    crate::op_id!("offset_half_height");

    fn name(&self) -> &'static str {
        "Offset Half Height"
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        *image = offset_wrap(image, 0, (image.height() / 2) as i64);
    }
}
