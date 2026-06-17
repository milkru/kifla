use eframe::egui;

use crate::operation::{par_pixels, Operation};
use crate::widgets;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ChannelMixer {
    red: [f32; 3],
    green: [f32; 3],
    blue: [f32; 3],
}

impl Default for ChannelMixer {
    fn default() -> Self {
        Self {
            red: [1.0, 0.0, 0.0],
            green: [0.0, 1.0, 0.0],
            blue: [0.0, 0.0, 1.0],
        }
    }
}

fn output_ui(ui: &mut egui::Ui, title: &str, weights: &mut [f32; 3]) -> bool {
    ui.label(title);
    let mut changed = false;
    changed |= widgets::slider(ui, "Red", &mut weights[0], -2.0..=2.0);
    changed |= widgets::slider(ui, "Green", &mut weights[1], -2.0..=2.0);
    changed |= widgets::slider(ui, "Blue", &mut weights[2], -2.0..=2.0);
    changed
}

impl Operation for ChannelMixer {
    crate::op_serde!("channel_mixer");

    fn name(&self) -> &'static str {
        "Channel Mixer"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        changed |= output_ui(ui, "Output Red", &mut self.red);
        ui.separator();
        changed |= output_ui(ui, "Output Green", &mut self.green);
        ui.separator();
        changed |= output_ui(ui, "Output Blue", &mut self.blue);
        changed
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        par_pixels(image, |px| {
            let r = px[0] as f32 / 255.0;
            let g = px[1] as f32 / 255.0;
            let b = px[2] as f32 / 255.0;

            let nr = self.red[0] * r + self.red[1] * g + self.red[2] * b;
            let ng = self.green[0] * r + self.green[1] * g + self.green[2] * b;
            let nb = self.blue[0] * r + self.blue[1] * g + self.blue[2] * b;

            px[0] = (nr.clamp(0.0, 1.0) * 255.0).round() as u8;
            px[1] = (ng.clamp(0.0, 1.0) * 255.0).round() as u8;
            px[2] = (nb.clamp(0.0, 1.0) * 255.0).round() as u8;
        });
    }
}
