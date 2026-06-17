use image::imageops;

use crate::operation::Operation;

pub struct Rotate90Cw;

impl Operation for Rotate90Cw {
    crate::op_id!("rotate_90_cw");

    fn name(&self) -> &'static str {
        "Rotate 90° CW"
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        *image = imageops::rotate90(image);
    }
}

pub struct Rotate90Ccw;

impl Operation for Rotate90Ccw {
    crate::op_id!("rotate_90_ccw");

    fn name(&self) -> &'static str {
        "Rotate 90° CCW"
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        *image = imageops::rotate270(image);
    }
}
