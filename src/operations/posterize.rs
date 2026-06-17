use eframe::egui;

use crate::operation::{par_pixels, Operation};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Posterize {
    levels: f32,
}

impl Default for Posterize {
    fn default() -> Self {
        Self { levels: 256.0 }
    }
}

impl Operation for Posterize {
    crate::op_serde!("posterize");

    fn name(&self) -> &'static str {
        "Posterize"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        if !ui.is_enabled() {
            ui.label(format!("Levels: {}", self.levels.round() as i32));
            return false;
        }
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Levels");
            let r = ui.add(
                egui::DragValue::new(&mut self.levels)
                    .clamp_range(2.0..=256.0)
                    .fixed_decimals(0)
                    .speed(1.0),
            );
            changed |= r.drag_released() || r.lost_focus();
        });
        changed
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        let steps = (self.levels.round() - 1.0).max(1.0);
        par_pixels(image, |px| {
            for channel in &mut px[..3] {
                let value = *channel as f32 / 255.0;
                let quantized = (value * steps).round() / steps;
                *channel = (quantized.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        });
    }
}
