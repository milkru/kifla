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

    fn gpu_pass(&self) -> Option<crate::gpu::GpuPass> {
        Some(crate::gpu::GpuPass::new(
            "flip_horizontal",
            r#"
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let dim = vec2<i32>(textureDimensions(tex));
    let coord = vec2<i32>(in.pos.xy);
    return textureLoad(tex, vec2<i32>(dim.x - 1 - coord.x, coord.y), 0);
}
"#,
        ))
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

    fn gpu_pass(&self) -> Option<crate::gpu::GpuPass> {
        Some(crate::gpu::GpuPass::new(
            "flip_vertical",
            r#"
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let dim = vec2<i32>(textureDimensions(tex));
    let coord = vec2<i32>(in.pos.xy);
    return textureLoad(tex, vec2<i32>(coord.x, dim.y - 1 - coord.y), 0);
}
"#,
        ))
    }
}
