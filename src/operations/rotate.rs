use image::imageops;

use crate::operation::Operation;

pub struct Rotate90Cw;

impl Operation for Rotate90Cw {
    fn name(&self) -> &'static str {
        "Rotate 90° CW"
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        *image = imageops::rotate90(image);
    }
}

pub struct Rotate90Ccw;

impl Operation for Rotate90Ccw {
    fn name(&self) -> &'static str {
        "Rotate 90° CCW"
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        *image = imageops::rotate270(image);
    }
}
