mod black_white;
mod brightness_contrast;
mod color_balance;
mod curves;
mod exposure;
mod hue_saturation;
mod invert;
mod levels;
mod vibrance;

pub use black_white::BlackWhite;
pub use brightness_contrast::BrightnessContrast;
pub use color_balance::ColorBalance;
pub use curves::Curves;
pub use exposure::Exposure;
pub use hue_saturation::HueSaturation;
pub use invert::Invert;
pub use levels::Levels;
pub use vibrance::Vibrance;

use crate::operation::OperationKind;

pub static OPERATION_GROUPS: &[&[OperationKind]] = &[
    &[
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
    &[
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
    ],
    &[OperationKind {
        menu_label: "🔄 Invert…",
        make: || Box::new(Invert),
    }],
];
