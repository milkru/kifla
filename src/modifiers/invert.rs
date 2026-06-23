use crate::modifier::Modifier;

pub struct Invert;

impl Modifier for Invert {
    crate::modifier_id!("invert");

    fn name(&self) -> &'static str {
        "Invert"
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
