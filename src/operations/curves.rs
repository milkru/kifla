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
        let y = eval_curve(points, i as f32 / 255.0).clamp(0.0, 1.0);
        *slot = (y * 255.0).round() as u8;
    }
    lut
}

fn eval_curve(points: &[egui::Pos2], x: f32) -> f32 {
    if points.is_empty() {
        return x;
    }
    if x <= points[0].x {
        return points[0].y;
    }
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if x <= b.x {
            let t = (x - a.x) / (b.x - a.x).max(1e-6);
            return a.y + t * (b.y - a.y);
        }
    }
    points[points.len() - 1].y
}
