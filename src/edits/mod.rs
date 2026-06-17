mod black_white;
mod brightness_contrast;
mod channel_mixer;
mod color_balance;
mod curves;
mod exposure;
mod flip;
mod hue_saturation;
mod indexed_color;
mod invert;
mod levels;
mod make_seamless;
mod offset;
mod posterize;
mod repeat;
mod resize;
mod rotate;
mod selective_color;
mod shadows_highlights;
mod splat;
mod threshold;
mod vibrance;

pub use black_white::BlackWhite;
pub use brightness_contrast::BrightnessContrast;
pub use channel_mixer::ChannelMixer;
pub use color_balance::ColorBalance;
pub use curves::Curves;
pub use exposure::Exposure;
pub use flip::{FlipHorizontal, FlipVertical};
pub use hue_saturation::HueSaturation;
pub use indexed_color::IndexedColor;
pub use invert::Invert;
pub use levels::Levels;
pub use make_seamless::MakeSeamless;
pub use offset::Offset;
pub use posterize::Posterize;
pub use repeat::Repeat;
pub use resize::Resize;
pub use rotate::{Rotate90Ccw, Rotate90Cw};
pub use selective_color::SelectiveColor;
pub use shadows_highlights::ShadowsHighlights;
pub use splat::Splat;
pub use threshold::Threshold;
pub use vibrance::Vibrance;

use crate::edit::{Edit, EditGroup, EditKind};

pub fn edit_from_json(id: &str, params: &serde_json::Value) -> Option<Box<dyn Edit>> {
    macro_rules! de {
        ($t:ty) => {
            serde_json::from_value::<$t>(params.clone())
                .ok()
                .map(|o| Box::new(o) as Box<dyn Edit>)
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
        "offset" => de!(Offset),
        "repeat" => de!(Repeat),
        "make_seamless" => de!(MakeSeamless),
        "splat" => de!(Splat),
        "resize" => de!(Resize),
        "invert" => Some(Box::new(Invert)),
        "flip_horizontal" => Some(Box::new(FlipHorizontal)),
        "flip_vertical" => Some(Box::new(FlipVertical)),
        "rotate_90_cw" => Some(Box::new(Rotate90Cw)),
        "rotate_90_ccw" => Some(Box::new(Rotate90Ccw)),
        _ => None,
    }
}

// Edits offered in the "Image" menu (color and tone adjustments).
pub static IMAGE_GROUPS: &[EditGroup] = &[
    EditGroup {
        label: "Tone",
        kinds: &[
            EditKind {
                menu_label: "☀ Brightness / Contrast…",
                make: || Box::new(BrightnessContrast::default()),
            },
            EditKind {
                menu_label: "📊 Levels…",
                make: || Box::new(Levels::default()),
            },
            EditKind {
                menu_label: "📈 Curves…",
                make: || Box::new(Curves::default()),
            },
            EditKind {
                menu_label: "🔆 Exposure…",
                make: || Box::new(Exposure::default()),
            },
        ],
    },
    EditGroup {
        label: "Color",
        kinds: &[
            EditKind {
                menu_label: "🎨 Hue / Saturation…",
                make: || Box::new(HueSaturation::default()),
            },
            EditKind {
                menu_label: "🌈 Vibrance…",
                make: || Box::new(Vibrance::default()),
            },
            EditKind {
                menu_label: "⚖ Color Balance…",
                make: || Box::new(ColorBalance::default()),
            },
            EditKind {
                menu_label: "🌓 Black & White…",
                make: || Box::new(BlackWhite::default()),
            },
            EditKind {
                menu_label: "🎛 Channel Mixer…",
                make: || Box::new(ChannelMixer::default()),
            },
        ],
    },
    EditGroup {
        label: "Stylize",
        kinds: &[
            EditKind {
                menu_label: "🎚 Posterize…",
                make: || Box::new(Posterize::default()),
            },
            EditKind {
                menu_label: "🔲 Threshold…",
                make: || Box::new(Threshold::default()),
            },
            EditKind {
                menu_label: "🎯 Selective Color…",
                make: || Box::new(SelectiveColor::default()),
            },
            EditKind {
                menu_label: "🗂 Indexed Color…",
                make: || Box::new(IndexedColor::default()),
            },
            EditKind {
                menu_label: "🔄 Invert",
                make: || Box::new(Invert),
            },
        ],
    },
    EditGroup {
        label: "Light",
        kinds: &[EditKind {
            menu_label: "🌗 Shadows / Highlights…",
            make: || Box::new(ShadowsHighlights::default()),
        }],
    },
];

// Edits offered in the "Transform" menu (geometry and tiling).
pub static TRANSFORM_GROUPS: &[EditGroup] = &[
    EditGroup {
        label: "Flip",
        kinds: &[
            EditKind {
                menu_label: "🔁 Flip Horizontal",
                make: || Box::new(FlipHorizontal),
            },
            EditKind {
                menu_label: "🔃 Flip Vertical",
                make: || Box::new(FlipVertical),
            },
        ],
    },
    EditGroup {
        label: "Rotate",
        kinds: &[
            EditKind {
                menu_label: "🔄 Rotate 90° CW",
                make: || Box::new(Rotate90Cw),
            },
            EditKind {
                menu_label: "🔄 Rotate 90° CCW",
                make: || Box::new(Rotate90Ccw),
            },
        ],
    },
    EditGroup {
        label: "Tiling",
        kinds: &[
            EditKind {
                menu_label: "🔀 Offset…",
                make: || Box::new(Offset::default()),
            },
            EditKind {
                menu_label: "🧱 Repeat…",
                make: || Box::new(Repeat::default()),
            },
            EditKind {
                menu_label: "🧩 Make Seamless…",
                make: || Box::new(MakeSeamless::default()),
            },
            EditKind {
                menu_label: "🎲 Splat…",
                make: || Box::new(Splat::default()),
            },
        ],
    },
    EditGroup {
        label: "Size",
        kinds: &[EditKind {
            menu_label: "📐 Resize…",
            make: || Box::new(Resize::default()),
        }],
    },
];
