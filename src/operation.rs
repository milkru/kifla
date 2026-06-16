use eframe::egui;
use rayon::prelude::*;

pub fn par_pixels(image: &mut image::RgbaImage, f: impl Fn(&mut [u8]) + Sync + Send) {
    let buffer: &mut [u8] = image;
    buffer.par_chunks_mut(4).for_each(f);
}

pub trait Operation {
    fn name(&self) -> &'static str;

    fn apply(&self, image: &mut image::RgbaImage);

    fn has_settings(&self) -> bool {
        false
    }

    fn settings_ui(&mut self, _ui: &mut egui::Ui) -> bool {
        false
    }

    fn on_added(&mut self, _width: u32, _height: u32) {}
}

pub struct OperationKind {
    pub menu_label: &'static str,
    pub make: fn() -> Box<dyn Operation>,
}

pub struct OperationGroup {
    pub label: &'static str,
    pub kinds: &'static [OperationKind],
}
