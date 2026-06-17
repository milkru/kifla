use image::imageops;

use crate::edit::Edit;

pub struct Rotate90Cw;

impl Edit for Rotate90Cw {
    crate::edit_id!("rotate_90_cw");

    fn name(&self) -> &'static str {
        "Rotate 90° CW"
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        *image = imageops::rotate90(image);
    }
}

pub struct Rotate90Ccw;

impl Edit for Rotate90Ccw {
    crate::edit_id!("rotate_90_ccw");

    fn name(&self) -> &'static str {
        "Rotate 90° CCW"
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        *image = imageops::rotate270(image);
    }
}
