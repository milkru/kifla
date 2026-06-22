use image::imageops;

use crate::modifier::Modifier;

pub struct FlipHorizontal;

impl Modifier for FlipHorizontal {
    crate::modifier_id!("flip_horizontal");

    fn name(&self) -> &'static str {
        "Flip Horizontal"
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        *image = imageops::flip_horizontal(image);
    }
}

pub struct FlipVertical;

impl Modifier for FlipVertical {
    crate::modifier_id!("flip_vertical");

    fn name(&self) -> &'static str {
        "Flip Vertical"
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        *image = imageops::flip_vertical(image);
    }
}
