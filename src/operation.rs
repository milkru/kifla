use eframe::egui;
use rayon::prelude::*;

pub fn par_pixels(image: &mut image::RgbaImage, f: impl Fn(&mut [u8]) + Sync + Send) {
    let buffer: &mut [u8] = image;
    buffer.par_chunks_mut(4).for_each(f);
}

pub trait Operation {
    fn name(&self) -> &'static str;

    /// Stable identifier used when saving/loading a stack. Must be unique and
    /// must never change for an existing operation.
    fn id(&self) -> &'static str;

    /// Serialize this operation's parameters. Parameterless operations keep the
    /// default (`null`).
    fn to_json(&self) -> serde_json::Value {
        serde_json::Value::Null
    }

    fn apply(&self, image: &mut image::RgbaImage);

    fn has_settings(&self) -> bool {
        false
    }

    fn settings_ui(&mut self, _ui: &mut egui::Ui) -> bool {
        false
    }

    fn on_added(&mut self, _width: u32, _height: u32) {}
}

/// Implements `id` for a parameterless operation.
#[macro_export]
macro_rules! op_id {
    ($id:literal) => {
        fn id(&self) -> &'static str {
            $id
        }
    };
}

/// Implements `id` and `to_json` for an operation whose struct derives `Serialize`.
#[macro_export]
macro_rules! op_serde {
    ($id:literal) => {
        fn id(&self) -> &'static str {
            $id
        }
        fn to_json(&self) -> serde_json::Value {
            serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
        }
    };
}

pub struct OperationKind {
    pub menu_label: &'static str,
    pub make: fn() -> Box<dyn Operation>,
}

pub struct OperationGroup {
    pub label: &'static str,
    pub kinds: &'static [OperationKind],
}
