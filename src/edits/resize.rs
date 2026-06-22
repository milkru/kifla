use eframe::egui;
use image::{imageops, Rgba, RgbaImage};
use rayon::prelude::*;

use crate::color;
use crate::edit::Edit;
use crate::widgets;

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
enum Sampling {
    Nearest,
    Bilinear,
    Bicubic,
    Lanczos,
    Min,
    Max,
}

impl Sampling {
    const ALL: [Sampling; 6] = [
        Sampling::Nearest,
        Sampling::Bilinear,
        Sampling::Bicubic,
        Sampling::Lanczos,
        Sampling::Min,
        Sampling::Max,
    ];

    fn name(self) -> &'static str {
        match self {
            Sampling::Nearest => "Nearest",
            Sampling::Bilinear => "Bilinear",
            Sampling::Bicubic => "Bicubic",
            Sampling::Lanczos => "Lanczos",
            Sampling::Min => "Min",
            Sampling::Max => "Max",
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Resize {
    width: u32,
    height: u32,
    sampling: Sampling,
    min_threshold: f32,
    max_threshold: f32,
}

impl Default for Resize {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            sampling: Sampling::Bicubic,
            min_threshold: 1.0,
            max_threshold: 0.0,
        }
    }
}

impl Edit for Resize {
    crate::edit_serde!("resize");

    fn name(&self) -> &'static str {
        "Resize"
    }

    fn has_settings(&self) -> bool {
        true
    }

    fn on_added(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) -> bool {
        if !ui.is_enabled() {
            ui.label(format!("Width: {}", self.width));
            ui.label(format!("Height: {}", self.height));
            ui.label(format!("Sampling: {}", self.sampling.name()));
            match self.sampling {
                Sampling::Min => ui.label(format!("Threshold: {:.2}", self.min_threshold)),
                Sampling::Max => ui.label(format!("Threshold: {:.2}", self.max_threshold)),
                _ => ui.label(""),
            };
            return false;
        }

        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Width");
            changed |= widgets::drag_value(ui, &mut self.width, 1..=16384);
        });
        ui.horizontal(|ui| {
            ui.label("Height");
            changed |= widgets::drag_value(ui, &mut self.height, 1..=16384);
        });
        ui.separator();
        egui::ComboBox::from_id_source("resize_sampling")
            .selected_text(self.sampling.name())
            .show_ui(ui, |ui| {
                for sampling in Sampling::ALL {
                    changed |= ui
                        .selectable_value(&mut self.sampling, sampling, sampling.name())
                        .changed();
                }
            });
        match self.sampling {
            Sampling::Min => {
                changed |= widgets::slider(ui, "Threshold", &mut self.min_threshold, 0.0..=1.0);
            }
            Sampling::Max => {
                changed |= widgets::slider(ui, "Threshold", &mut self.max_threshold, 0.0..=1.0);
            }
            _ => {}
        }
        changed
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        let width = self.width.max(1);
        let height = self.height.max(1);
        if width == image.width() && height == image.height() {
            return;
        }
        *image = match self.sampling {
            Sampling::Nearest => {
                imageops::resize(image, width, height, imageops::FilterType::Nearest)
            }
            Sampling::Bilinear => {
                imageops::resize(image, width, height, imageops::FilterType::Triangle)
            }
            Sampling::Bicubic => {
                imageops::resize(image, width, height, imageops::FilterType::CatmullRom)
            }
            Sampling::Lanczos => {
                imageops::resize(image, width, height, imageops::FilterType::Lanczos3)
            }
            Sampling::Min => resample_extreme(image, width, height, false, self.min_threshold),
            Sampling::Max => resample_extreme(image, width, height, true, self.max_threshold),
        };
    }
}

fn luminance(pixel: &Rgba<u8>) -> f32 {
    color::luma(pixel[0] as f32, pixel[1] as f32, pixel[2] as f32)
}

fn resample_extreme(
    src: &RgbaImage,
    dst_w: u32,
    dst_h: u32,
    take_max: bool,
    threshold: f32,
) -> RgbaImage {
    let (src_w, src_h) = (src.width(), src.height());
    let mut out = RgbaImage::new(dst_w, dst_h);
    let threshold = threshold * 255.0;

    let span = |index: u32, dst: u32, src: u32| {
        let lo = (index as u64 * src as u64 / dst as u64) as u32;
        let hi = ((index as u64 + 1) * src as u64).div_ceil(dst as u64) as u32;
        (lo, hi.max(lo + 1).min(src))
    };

    let nearest = |index: u32, dst: u32, src: u32| {
        (((index as u64 * 2 + 1) * src as u64) / (dst as u64 * 2)).min(src as u64 - 1) as u32
    };

    let row_len = dst_w as usize * 4;
    let buffer: &mut [u8] = &mut out;
    buffer
        .par_chunks_mut(row_len)
        .enumerate()
        .for_each(|(oy, row)| {
            let oy = oy as u32;
            let (y0, y1) = span(oy, dst_h, src_h);
            let ny = nearest(oy, dst_h, src_h);
            for ox in 0..dst_w {
                let (x0, x1) = span(ox, dst_w, src_w);
                let mut eligible: Option<(Rgba<u8>, f32)> = None;
                for yy in y0..y1 {
                    for xx in x0..x1 {
                        let pixel = src.get_pixel(xx, yy);
                        let lum = luminance(pixel);
                        let passes = if take_max {
                            lum >= threshold
                        } else {
                            lum <= threshold
                        };
                        if !passes {
                            continue;
                        }
                        let better = match eligible {
                            None => true,
                            Some((_, best_lum)) => {
                                if take_max {
                                    lum > best_lum
                                } else {
                                    lum < best_lum
                                }
                            }
                        };
                        if better {
                            eligible = Some((*pixel, lum));
                        }
                    }
                }
                let pixel = eligible
                    .map(|(p, _)| p)
                    .unwrap_or_else(|| *src.get_pixel(nearest(ox, dst_w, src_w), ny));
                let o = ox as usize * 4;
                row[o..o + 4].copy_from_slice(&pixel.0);
            }
        });

    out
}
