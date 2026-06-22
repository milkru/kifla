use crate::modifier::Modifier;
use crate::pixel::par_pixels;

pub struct Invert;

impl Modifier for Invert {
    crate::modifier_id!("invert");

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
