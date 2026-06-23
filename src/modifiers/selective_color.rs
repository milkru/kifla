use eframe::egui;

use crate::modifier::Modifier;
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

    fn gpu_pass(&self) -> Option<crate::gpu::GpuPass> {
        let family = Family::ALL.iter().position(|&f| f == self.family).unwrap() as f32;
        Some(
            crate::gpu::GpuPass::new(
                "selective_color",
                r#"
struct P { v: array<vec4<f32>, 2> };
@group(0) @binding(2) var<uniform> p: P;

fn hue_weight(h: f32, s: f32, center: f32) -> f32 {
    var d = abs(h - center);
    d = min(d, 1.0 - d);
    return max(1.0 - d / (1.0 / 6.0), 0.0) * s;
}

fn family_weight(family: i32, h: f32, s: f32, l: f32) -> f32 {
    switch family {
        case 0: { return hue_weight(h, s, 0.0); }
        case 1: { return hue_weight(h, s, 1.0 / 6.0); }
        case 2: { return hue_weight(h, s, 2.0 / 6.0); }
        case 3: { return hue_weight(h, s, 3.0 / 6.0); }
        case 4: { return hue_weight(h, s, 4.0 / 6.0); }
        case 5: { return hue_weight(h, s, 5.0 / 6.0); }
        case 6: { return clamp((l - 0.7) / 0.3, 0.0, 1.0); }
        case 8: { return clamp((0.3 - l) / 0.3, 0.0, 1.0); }
        default: { return (1.0 - s) * (1.0 - abs(2.0 * l - 1.0)); }
    }
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureLoad(tex, vec2<i32>(in.pos.xy), 0);
    let family = i32(p.v[0].x);
    let cyan = p.v[0].y;
    let magenta = p.v[0].z;
    let yellow = p.v[0].w;
    let black = p.v[1].x;
    let hsl = rgb_to_hsl(c.rgb);
    let w = family_weight(family, hsl.x, hsl.y, hsl.z);
    let rgb = clamp(
        c.rgb - w * vec3<f32>(cyan + black, magenta + black, yellow + black),
        vec3<f32>(0.0),
        vec3<f32>(1.0),
    );
    return vec4<f32>(rgb, c.a);
}
"#,
            )
            .with_uniforms(&crate::gpu::uniforms(&[
                family,
                self.cyan,
                self.magenta,
                self.yellow,
                self.black,
            ])),
        )
    }
}
