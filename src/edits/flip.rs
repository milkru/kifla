use image::imageops;

use crate::edit::Edit;

pub struct FlipHorizontal;

impl Edit for FlipHorizontal {
    crate::edit_id!("flip_horizontal");

    fn name(&self) -> &'static str {
        "Flip Horizontal"
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        *image = imageops::flip_horizontal(image);
    }
}

pub struct FlipVertical;

impl Edit for FlipVertical {
    crate::edit_id!("flip_vertical");

    fn name(&self) -> &'static str {
        "Flip Vertical"
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        *image = imageops::flip_vertical(image);
    }
}
