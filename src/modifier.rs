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

    /// The fragment pass this modifier runs. Single-pass modifiers implement
    /// this; multi-pass ones override [`gpu_passes`](Modifier::gpu_passes) and
    /// compute-based ones override [`gpu_step`](Modifier::gpu_step).
    fn gpu_pass(&self) -> Option<crate::gpu::GpuPass> {
        None
    }

    /// The full sequence of GPU passes this modifier runs (a "group"). Defaults
    /// to the single [`gpu_pass`](Modifier::gpu_pass). Within the group, each
    /// pass reads the previous output (binding 0) and the group's input
    /// (binding 3).
    fn gpu_passes(&self) -> Vec<crate::gpu::GpuPass> {
        self.gpu_pass().into_iter().collect()
    }

    /// The modifier's GPU work as a [`GpuStep`]. Defaults to wrapping
    /// [`gpu_passes`](Modifier::gpu_passes); modifiers needing compute (e.g.
    /// indexed color) override this directly.
    fn gpu_step(&self) -> crate::gpu::GpuStep {
        crate::gpu::GpuStep::Fragment(self.gpu_passes())
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
