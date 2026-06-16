use image::imageops;

use crate::operation::Operation;

pub struct FlipHorizontal;

impl Operation for FlipHorizontal {
    fn name(&self) -> &'static str {
        "Flip Horizontal"
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        *image = imageops::flip_horizontal(image);
    }
}

pub struct FlipVertical;

impl Operation for FlipVertical {
    fn name(&self) -> &'static str {
        "Flip Vertical"
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        *image = imageops::flip_vertical(image);
    }
}
