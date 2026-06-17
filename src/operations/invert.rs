use crate::operation::{par_pixels, Operation};

pub struct Invert;

impl Operation for Invert {
    crate::op_id!("invert");

    fn name(&self) -> &'static str {
        "Invert"
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        par_pixels(image, |px| {
            px[0] = 255 - px[0];
            px[1] = 255 - px[1];
            px[2] = 255 - px[2];
        });
    }
}
