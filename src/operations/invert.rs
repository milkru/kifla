use crate::operation::Operation;

pub struct Invert;

impl Operation for Invert {
    fn name(&self) -> &'static str {
        "Invert"
    }

    fn apply(&self, image: &mut image::RgbaImage) {
        for pixel in image.pixels_mut() {
            pixel[0] = 255 - pixel[0];
            pixel[1] = 255 - pixel[1];
            pixel[2] = 255 - pixel[2];
        }
    }
}
