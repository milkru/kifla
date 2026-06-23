mod black_white;
mod blend;
mod brightness_contrast;
mod channel_mixer;
mod color_balance;
mod crop;
mod curves;
mod exposure;
mod flip;
mod hue_saturation;
mod indexed_color;
mod invert;
mod levels;
mod lighting;
mod offset;
mod posterize;
mod repeat;
mod resize;
mod rotate;
mod selective_color;
mod shadows_highlights;
mod skew;
mod threshold;
mod vibrance;

pub use black_white::BlackWhite;
pub use blend::Blend;
pub use brightness_contrast::BrightnessContrast;
pub use channel_mixer::ChannelMixer;
pub use color_balance::ColorBalance;
pub use crop::Crop;
pub use curves::Curves;
pub use exposure::Exposure;
pub use flip::{FlipHorizontal, FlipVertical};
pub use hue_saturation::HueSaturation;
pub use indexed_color::IndexedColor;
pub use invert::Invert;
pub use levels::Levels;
pub use lighting::Lighting;
pub use offset::Offset;
pub use posterize::Posterize;
pub use repeat::Repeat;
pub use resize::Resize;
pub use rotate::{Rotate, Rotate90Ccw, Rotate90Cw};
pub use selective_color::SelectiveColor;
pub use shadows_highlights::ShadowsHighlights;
pub use skew::Skew;
pub use threshold::Threshold;
pub use vibrance::Vibrance;

use crate::modifier::{Modifier, ModifierGroup, ModifierKind};

pub fn modifier_from_json(id: &str, params: &serde_json::Value) -> Option<Box<dyn Modifier>> {
    macro_rules! de {
        ($t:ty) => {
            serde_json::from_value::<$t>(params.clone())
                .ok()
                .map(|o| Box::new(o) as Box<dyn Modifier>)
        };
    }
    match id {
        "brightness_contrast" => de!(BrightnessContrast),
        "levels" => de!(Levels),
        "curves" => de!(Curves),
        "exposure" => de!(Exposure),
        "hue_saturation" => de!(HueSaturation),
        "vibrance" => de!(Vibrance),
        "color_balance" => de!(ColorBalance),
        "black_white" => de!(BlackWhite),
        "channel_mixer" => de!(ChannelMixer),
        "posterize" => de!(Posterize),
        "threshold" => de!(Threshold),
        "selective_color" => de!(SelectiveColor),
        "indexed_color" => de!(IndexedColor),
        "shadows_highlights" => de!(ShadowsHighlights),
        "lighting" => de!(Lighting),
        "offset" => de!(Offset),
        "repeat" => de!(Repeat),
        "blend" | "make_seamless" => de!(Blend),
        "skew" => de!(Skew),
        "rotate" => de!(Rotate),
        "resize" => de!(Resize),
        "crop" => de!(Crop),
        "invert" => Some(Box::new(Invert)),
        "flip_horizontal" => Some(Box::new(FlipHorizontal)),
        "flip_vertical" => Some(Box::new(FlipVertical)),
        "rotate_90_cw" => Some(Box::new(Rotate90Cw)),
        "rotate_90_ccw" => Some(Box::new(Rotate90Ccw)),
        _ => None,
    }
}

/// A fresh, default-valued modifier matching `id`, by scanning the menu
/// registries for the factory that produces it. Used to reset a modifier in the
/// stack back to its defaults.
pub fn default_modifier(id: &str) -> Option<Box<dyn Modifier>> {
    TRANSFORM_GROUPS
        .iter()
        .chain(IMAGE_GROUPS.iter())
        .flat_map(|group| group.kinds.iter())
        .map(|kind| (kind.make)())
        .find(|modifier| modifier.id() == id)
}

// Modifiers offered in the "Image" menu (color and tone adjustments).
pub static IMAGE_GROUPS: &[ModifierGroup] = &[
    ModifierGroup {
        label: "Tone",
        kinds: &[
            ModifierKind {
                menu_label: "☀ Brightness / Contrast…",
                make: || Box::new(BrightnessContrast::default()),
            },
            ModifierKind {
                menu_label: "📊 Levels…",
                make: || Box::new(Levels::default()),
            },
            ModifierKind {
                menu_label: "📈 Curves…",
                make: || Box::new(Curves::default()),
            },
            ModifierKind {
                menu_label: "🔆 Exposure…",
                make: || Box::new(Exposure::default()),
            },
        ],
    },
    ModifierGroup {
        label: "Color",
        kinds: &[
            ModifierKind {
                menu_label: "🎨 Hue / Saturation…",
                make: || Box::new(HueSaturation::default()),
            },
            ModifierKind {
                menu_label: "🌈 Vibrance…",
                make: || Box::new(Vibrance::default()),
            },
            ModifierKind {
                menu_label: "⚖ Color Balance…",
                make: || Box::new(ColorBalance::default()),
            },
            ModifierKind {
                menu_label: "🌓 Black & White…",
                make: || Box::new(BlackWhite::default()),
            },
            ModifierKind {
                menu_label: "🎛 Channel Mixer…",
                make: || Box::new(ChannelMixer::default()),
            },
        ],
    },
    ModifierGroup {
        label: "Stylize",
        kinds: &[
            ModifierKind {
                menu_label: "🎚 Posterize…",
                make: || Box::new(Posterize::default()),
            },
            ModifierKind {
                menu_label: "🔲 Threshold…",
                make: || Box::new(Threshold::default()),
            },
            ModifierKind {
                menu_label: "🎯 Selective Color…",
                make: || Box::new(SelectiveColor::default()),
            },
            ModifierKind {
                menu_label: "🗂 Indexed Color…",
                make: || Box::new(IndexedColor::default()),
            },
            ModifierKind {
                menu_label: "🔄 Invert",
                make: || Box::new(Invert),
            },
        ],
    },
    ModifierGroup {
        label: "Light",
        kinds: &[
            ModifierKind {
                menu_label: "🌗 Shadows / Highlights…",
                make: || Box::new(ShadowsHighlights::default()),
            },
            ModifierKind {
                menu_label: "💡 Lighting Normalization…",
                make: || Box::new(Lighting::default()),
            },
        ],
    },
];

// Modifiers offered in the "Transform" menu (geometry and tiling).
pub static TRANSFORM_GROUPS: &[ModifierGroup] = &[
    ModifierGroup {
        label: "Flip",
        kinds: &[
            ModifierKind {
                menu_label: "🔁 Flip Horizontal",
                make: || Box::new(FlipHorizontal),
            },
            ModifierKind {
                menu_label: "🔃 Flip Vertical",
                make: || Box::new(FlipVertical),
            },
        ],
    },
    ModifierGroup {
        label: "Rotate",
        kinds: &[
            ModifierKind {
                menu_label: "🔄 Rotate 90° CW",
                make: || Box::new(Rotate90Cw),
            },
            ModifierKind {
                menu_label: "🔄 Rotate 90° CCW",
                make: || Box::new(Rotate90Ccw),
            },
        ],
    },
    ModifierGroup {
        label: "Tiling",
        kinds: &[
            ModifierKind {
                menu_label: "🔀 Offset…",
                make: || Box::new(Offset::default()),
            },
            ModifierKind {
                menu_label: "🧱 Repeat…",
                make: || Box::new(Repeat::default()),
            },
            ModifierKind {
                menu_label: "🌫 Blend…",
                make: || Box::new(Blend::default()),
            },
            ModifierKind {
                menu_label: "🌀 Rotate…",
                make: || Box::new(Rotate::default()),
            },
            ModifierKind {
                menu_label: "🔺 Skew…",
                make: || Box::new(Skew::default()),
            },
        ],
    },
    ModifierGroup {
        label: "Size",
        kinds: &[
            ModifierKind {
                menu_label: "📐 Resize…",
                make: || Box::new(Resize::default()),
            },
            ModifierKind {
                menu_label: "✂ Crop…",
                make: || Box::new(Crop::default()),
            },
        ],
    },
];
