mod brightness_contrast;
mod curves;
mod exposure;
mod invert;
mod levels;

pub use brightness_contrast::BrightnessContrast;
pub use curves::Curves;
pub use exposure::Exposure;
pub use invert::Invert;
pub use levels::Levels;

use crate::operation::OperationKind;

pub static OPERATION_GROUPS: &[&[OperationKind]] = &[
    &[
        OperationKind {
            menu_label: "☀ Brightness / Contrast",
            make: || Box::new(BrightnessContrast::default()),
        },
        OperationKind {
            menu_label: "📊 Levels",
            make: || Box::new(Levels::default()),
        },
        OperationKind {
            menu_label: "📈 Curves",
            make: || Box::new(Curves::default()),
        },
        OperationKind {
            menu_label: "🔆 Exposure",
            make: || Box::new(Exposure::default()),
        },
    ],
    &[OperationKind {
        menu_label: "🔄 Invert",
        make: || Box::new(Invert),
    }],
];
