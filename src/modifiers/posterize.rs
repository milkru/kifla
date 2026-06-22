use eframe::egui;

use crate::modifier::Modifier;
use crate::pixel::map_rgb;
use crate::widgets;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Posterize {
    levels: f32,
}

impl Default for Posterize {
    fn default() -> Self {
        Self { levels: 256.0 }
    }
}

impl Modifier for Posterize {
    crate::modifier_serde!("posterize");

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
            changed |= widgets::fine_tune(ui, &r, &mut self.levels, 2.0..=256.0);
        });
        changed
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        let steps = (self.levels.round() - 1.0).max(1.0);
        map_rgb(image, |value| (value * steps).round() / steps);
    }
}
