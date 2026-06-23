use eframe::egui;

use crate::color;
use crate::modifier::Modifier;
use crate::pixel::{par_pixels, to_u8};
use crate::widgets;

#[derive(Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
enum Family {
    #[default]
    Reds,
    Yellows,
    Greens,
    Cyans,
    Blues,
    Magentas,
    Whites,
    Neutrals,
    Blacks,
}

impl Family {
    const ALL: [Family; 9] = [
        Family::Reds,
        Family::Yellows,
        Family::Greens,
        Family::Cyans,
        Family::Blues,
        Family::Magentas,
        Family::Whites,
        Family::Neutrals,
        Family::Blacks,
    ];

    fn name(self) -> &'static str {
        match self {
            Family::Reds => "Reds",
            Family::Yellows => "Yellows",
            Family::Greens => "Greens",
            Family::Cyans => "Cyans",
            Family::Blues => "Blues",
            Family::Magentas => "Magentas",
            Family::Whites => "Whites",
            Family::Neutrals => "Neutrals",
            Family::Blacks => "Blacks",
        }
    }

    fn weight(self, h: f32, s: f32, l: f32) -> f32 {
        let hue_weight = |center: f32| {
            let d = (h - center).abs();
            let d = d.min(1.0 - d);
            (1.0 - d / (1.0 / 6.0)).max(0.0) * s
        };
        match self {
            Family::Reds => hue_weight(0.0),
            Family::Yellows => hue_weight(1.0 / 6.0),
            Family::Greens => hue_weight(2.0 / 6.0),
            Family::Cyans => hue_weight(3.0 / 6.0),
            Family::Blues => hue_weight(4.0 / 6.0),
            Family::Magentas => hue_weight(5.0 / 6.0),
            Family::Whites => ((l - 0.7) / 0.3).clamp(0.0, 1.0),
            Family::Blacks => ((0.3 - l) / 0.3).clamp(0.0, 1.0),
            Family::Neutrals => (1.0 - s) * (1.0 - (2.0 * l - 1.0).abs()),
        }
    }
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct SelectiveColor {
    family: Family,
    cyan: f32,
    magenta: f32,
    yellow: f32,
    black: f32,
}

impl Modifier for SelectiveColor {
    crate::modifier_serde!("selective_color");

    fn name(&self) -> &'static str {
        "Selective Color"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        if ui.is_enabled() {
            let combo = egui::ComboBox::from_id_source("selective_color_family")
                .selected_text(self.family.name())
                .show_ui(ui, |ui| {
                    for family in Family::ALL {
                        changed |= ui
                            .selectable_value(&mut self.family, family, family.name())
                            .changed();
                    }
                });
            changed |= widgets::combo_scroll(ui, &combo.response, &mut self.family, &Family::ALL);
        } else {
            ui.label(format!("Colors: {}", self.family.name()));
        }
        changed |= widgets::slider(ui, "Cyan", &mut self.cyan, -1.0..=1.0);
        changed |= widgets::slider(ui, "Magenta", &mut self.magenta, -1.0..=1.0);
        changed |= widgets::slider(ui, "Yellow", &mut self.yellow, -1.0..=1.0);
        changed |= widgets::slider(ui, "Black", &mut self.black, -1.0..=1.0);
        changed
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        par_pixels(image, |px| {
            let r = px[0] as f32 / 255.0;
            let g = px[1] as f32 / 255.0;
            let b = px[2] as f32 / 255.0;

            let (h, s, l) = color::rgb_to_hsl(r, g, b);
            let w = self.family.weight(h, s, l);

            px[0] = to_u8(r - w * (self.cyan + self.black));
            px[1] = to_u8(g - w * (self.magenta + self.black));
            px[2] = to_u8(b - w * (self.yellow + self.black));
        });
    }
}
