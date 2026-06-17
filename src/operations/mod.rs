mod black_white;
mod brightness_contrast;
mod channel_mixer;
mod color_balance;
mod curves;
mod exposure;
mod flip;
mod hue_saturation;
mod invert;
mod levels;
mod offset;
mod posterize;
mod resize;
mod rotate;
mod selective_color;
mod shadows_highlights;
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
pub use invert::Invert;
pub use levels::Levels;
pub use offset::{Offset, OffsetHalfHeight, OffsetHalfWidth};
pub use posterize::Posterize;
pub use resize::Resize;
pub use rotate::{Rotate90Ccw, Rotate90Cw};
pub use selective_color::SelectiveColor;
pub use shadows_highlights::ShadowsHighlights;
pub use threshold::Threshold;
pub use vibrance::Vibrance;

use crate::operation::{OperationGroup, OperationKind};

pub static OPERATION_GROUPS: &[OperationGroup] = &[
    OperationGroup {
        label: "Tone",
        kinds: &[
            OperationKind {
                menu_label: "☀ Brightness / Contrast…",
                make: || Box::new(BrightnessContrast::default()),
            },
            OperationKind {
                menu_label: "📊 Levels…",
                make: || Box::new(Levels::default()),
            },
            OperationKind {
                menu_label: "📈 Curves…",
                make: || Box::new(Curves::default()),
            },
            OperationKind {
                menu_label: "🔆 Exposure…",
                make: || Box::new(Exposure::default()),
            },
        ],
    },
    OperationGroup {
        label: "Color",
        kinds: &[
            OperationKind {
                menu_label: "🎨 Hue / Saturation…",
                make: || Box::new(HueSaturation::default()),
            },
            OperationKind {
                menu_label: "🌈 Vibrance…",
                make: || Box::new(Vibrance::default()),
            },
            OperationKind {
                menu_label: "⚖ Color Balance…",
                make: || Box::new(ColorBalance::default()),
            },
            OperationKind {
                menu_label: "🌓 Black & White…",
                make: || Box::new(BlackWhite::default()),
            },
            OperationKind {
                menu_label: "🎛 Channel Mixer…",
                make: || Box::new(ChannelMixer::default()),
            },
        ],
    },
    OperationGroup {
        label: "Stylize",
        kinds: &[
            OperationKind {
                menu_label: "🎚 Posterize…",
                make: || Box::new(Posterize::default()),
            },
            OperationKind {
                menu_label: "🔲 Threshold…",
                make: || Box::new(Threshold::default()),
            },
            OperationKind {
                menu_label: "🎯 Selective Color…",
                make: || Box::new(SelectiveColor::default()),
            },
            OperationKind {
                menu_label: "🔄 Invert…",
                make: || Box::new(Invert),
            },
        ],
    },
    OperationGroup {
        label: "Light",
        kinds: &[OperationKind {
            menu_label: "🌗 Shadows / Highlights…",
            make: || Box::new(ShadowsHighlights::default()),
        }],
    },
];

pub static TRANSFORM_GROUPS: &[OperationGroup] = &[
    OperationGroup {
        label: "Flip",
        kinds: &[
            OperationKind {
                menu_label: "🔁 Flip Horizontal…",
                make: || Box::new(FlipHorizontal),
            },
            OperationKind {
                menu_label: "🔃 Flip Vertical…",
                make: || Box::new(FlipVertical),
            },
        ],
    },
    OperationGroup {
        label: "Rotate",
        kinds: &[
            OperationKind {
                menu_label: "🔄 Rotate 90° CW…",
                make: || Box::new(Rotate90Cw),
            },
            OperationKind {
                menu_label: "🔄 Rotate 90° CCW…",
                make: || Box::new(Rotate90Ccw),
            },
        ],
    },
    OperationGroup {
        label: "Offset",
        kinds: &[
            OperationKind {
                menu_label: "🔀 Offset…",
                make: || Box::new(Offset::default()),
            },
            OperationKind {
                menu_label: "🔀 Offset Half Width…",
                make: || Box::new(OffsetHalfWidth),
            },
            OperationKind {
                menu_label: "🔀 Offset Half Height…",
                make: || Box::new(OffsetHalfHeight),
            },
        ],
    },
    OperationGroup {
        label: "Size",
        kinds: &[OperationKind {
            menu_label: "📐 Resize…",
            make: || Box::new(Resize::default()),
        }],
    },
];
