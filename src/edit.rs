use eframe::egui;

pub trait Edit {
    fn name(&self) -> &'static str;

    /// Stable identifier used when saving/loading a stack. Must be unique and
    /// must never change for an existing edit.
    fn id(&self) -> &'static str;

    /// Serialize this edit's parameters. Parameterless edits keep the default
    /// (`null`).
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

/// Implements `id` for a parameterless edit.
#[macro_export]
macro_rules! edit_id {
    ($id:literal) => {
        fn id(&self) -> &'static str {
            $id
        }
    };
}

/// Implements `id` and `to_json` for an edit whose struct derives `Serialize`.
#[macro_export]
macro_rules! edit_serde {
    ($id:literal) => {
        fn id(&self) -> &'static str {
            $id
        }
        fn to_json(&self) -> serde_json::Value {
            serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
        }
    };
}

pub struct EditKind {
    pub menu_label: &'static str,
    pub make: fn() -> Box<dyn Edit>,
}

pub struct EditGroup {
    pub label: &'static str,
    pub kinds: &'static [EditKind],
}
