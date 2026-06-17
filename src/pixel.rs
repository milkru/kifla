use rayon::prelude::*;

/// Maps a 0..1 value back to a clamped 8-bit channel.
pub fn to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Runs `f` over every pixel (as a 4-byte RGBA slice) in parallel.
pub fn par_pixels(image: &mut image::RgbaImage, f: impl Fn(&mut [u8]) + Sync + Send) {
    let buffer: &mut [u8] = image;
    buffer.par_chunks_mut(4).for_each(f);
}

/// Applies `f` to each RGB channel independently, working in 0..1 space and
/// leaving alpha untouched.
pub fn map_rgb(image: &mut image::RgbaImage, f: impl Fn(f32) -> f32 + Sync + Send) {
    par_pixels(image, |px| {
        for channel in &mut px[..3] {
            *channel = to_u8(f(*channel as f32 / 255.0));
        }
    });
}
