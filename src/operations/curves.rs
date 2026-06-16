use eframe::egui;

use crate::operation::Operation;
use crate::widgets;

pub struct Curves {
    points: Vec<egui::Pos2>,
}

impl Default for Curves {
    fn default() -> Self {
        Self {
            points: vec![egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)],
        }
    }
}

impl Operation for Curves {
    fn name(&self) -> &'static str {
        "Curves"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        widgets::curve_editor(ui, &mut self.points)
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        let lut = build_lut(&self.points);
        for pixel in image.pixels_mut() {
            for channel in &mut pixel.0[..3] {
                *channel = lut[*channel as usize];
            }
        }
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
