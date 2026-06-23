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

    fn gpu_pass(&self) -> Option<crate::gpu::GpuPass> {
        Some(crate::gpu::GpuPass::new(
            "invert",
            r#"
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureLoad(tex, vec2<i32>(in.pos.xy), 0);
    return vec4<f32>(1.0 - c.rgb, c.a);
}
"#,
        ))
    }
}
