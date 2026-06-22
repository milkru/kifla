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

/// Bilinear sample of a wrapped (tiling) source buffer; integer coordinates
/// land on pixel centers. Returns the four channels in 0..255 float space.
pub fn sample_wrap(src: &[u8], x: f32, y: f32, w: u32, h: u32) -> [f32; 4] {
    let xf = x.rem_euclid(w as f32);
    let yf = y.rem_euclid(h as f32);
    let x0 = (xf.floor() as u32) % w;
    let y0 = (yf.floor() as u32) % h;
    let x1 = (x0 + 1) % w;
    let y1 = (y0 + 1) % h;
    let tx = xf - xf.floor();
    let ty = yf - yf.floor();
    let idx = |x: u32, y: u32| ((y * w + x) as usize) * 4;
    let (i00, i10, i01, i11) = (idx(x0, y0), idx(x1, y0), idx(x0, y1), idx(x1, y1));
    let mut out = [0.0; 4];
    for c in 0..4 {
        let a = src[i00 + c] as f32 * (1.0 - tx) + src[i10 + c] as f32 * tx;
        let b = src[i01 + c] as f32 * (1.0 - tx) + src[i11 + c] as f32 * tx;
        out[c] = a * (1.0 - ty) + b * ty;
    }
    out
}

/// Resamples `image` in place: each output pixel (ox, oy) is filled by
/// bilinearly sampling the original at the source coordinate returned by
/// `map`, wrapping around the edges so the result stays tileable.
pub fn remap_wrap(image: &mut image::RgbaImage, map: impl Fn(f32, f32) -> (f32, f32) + Sync + Send) {
    let (w, h) = (image.width(), image.height());
    if w == 0 || h == 0 {
        return;
    }
    let src = image.clone();
    let src_raw: &[u8] = &src;
    let row_len = w as usize * 4;
    let buffer: &mut [u8] = image;
    buffer
        .par_chunks_mut(row_len)
        .enumerate()
        .for_each(|(oy, row)| {
            let oy = oy as f32;
            for ox in 0..w {
                let (sx, sy) = map(ox as f32, oy);
                let s = sample_wrap(src_raw, sx, sy, w, h);
                let o = ox as usize * 4;
                for c in 0..4 {
                    row[o + c] = s[c].round().clamp(0.0, 255.0) as u8;
                }
            }
        });
}
