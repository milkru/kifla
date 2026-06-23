use eframe::egui;

pub trait Modifier {
    fn name(&self) -> &'static str;

    /// Stable identifier used when saving/loading a stack. Must be unique and
    /// must never change for an existing modifier.
    fn id(&self) -> &'static str;

    /// Serialize this modifier's parameters. Parameterless modifiers keep the default
    /// (`null`).
    fn to_json(&self) -> serde_json::Value {
        serde_json::Value::Null
    }

    fn apply(&self, image: &mut image::RgbaImage);

    /// Optional GPU implementation: the fragment passes this modifier runs.
    /// `None` (the default) means CPU-only - a stack is run on the GPU only when
    /// every enabled modifier provides a pass, otherwise it falls back to
    /// [`apply`](Modifier::apply).
    fn gpu_pass(&self) -> Option<crate::gpu::GpuPass> {
        None
    }

    fn has_settings(&self) -> bool {
        false
    }

    fn settings_ui(&mut self, _ui: &mut egui::Ui) -> bool {
        false
    }

    fn on_added(&mut self, _width: u32, _height: u32) {}
}

/// Implements `id` for a parameterless modifier.
#[macro_export]
macro_rules! modifier_id {
    ($id:literal) => {
        fn id(&self) -> &'static str {
            $id
        }
    };
}

/// Implements `id` and `to_json` for an modifier whose struct derives `Serialize`.
#[macro_export]
macro_rules! modifier_serde {
    ($id:literal) => {
        fn id(&self) -> &'static str {
            $id
        }
        fn to_json(&self) -> serde_json::Value {
            serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
        }
    };
}

pub struct ModifierKind {
    pub menu_label: &'static str,
    pub make: fn() -> Box<dyn Modifier>,
}

pub struct ModifierGroup {
    pub label: &'static str,
    pub kinds: &'static [ModifierKind],
}
