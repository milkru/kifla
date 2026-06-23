use eframe::egui;

use crate::modifier::Modifier;
use crate::widgets;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct IndexedColor {
    colors: u32,
    dither: bool,
    amount: f32,
}

impl Default for IndexedColor {
    fn default() -> Self {
        Self {
            colors: 256,
            dither: false,
            amount: 1.0,
        }
    }
}

impl Modifier for IndexedColor {
    crate::modifier_serde!("indexed_color");

    fn name(&self) -> &'static str {
        "Indexed Color"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        if !ui.is_enabled() {
            ui.label(format!("Colors: {}", self.colors));
            ui.label(format!(
                "Dither: {}",
                if self.dither { "on" } else { "off" }
            ));
            if self.dither {
                ui.label(format!("Amount: {:.2}", self.amount));
            }
            return false;
        }

        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Colors");
            changed |= widgets::drag_value(ui, &mut self.colors, 2..=256);
        });
        changed |= ui.checkbox(&mut self.dither, "Dither").changed();
        if self.dither {
            changed |= widgets::slider(ui, "Amount", &mut self.amount, 0.0..=1.0);
        }
        changed
    }

    fn gpu_step(&self) -> crate::gpu::GpuStep {
        let n = self.colors.clamp(2, 256);
        // A full 256-colour palette with no dithering leaves the image unchanged.
        if n >= 256 && !self.dither {
            return crate::gpu::GpuStep::Fragment(Vec::new());
        }
        crate::gpu::GpuStep::IndexColor {
            colors: n,
            dither: self.dither,
            amount: self.amount,
        }
    }
}
