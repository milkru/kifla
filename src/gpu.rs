use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use eframe::wgpu;

const TEX_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

// Shared vertex stage + input bindings prepended to every modifier fragment
// shader. A single fullscreen triangle covers the target; `uv` has its origin
// at the top-left to match image-pixel order. Per-pixel shaders read exact
// texels with `textureLoad` (no filtering); geometry shaders use `samp`.
const SHADER_HEADER: &str = r#"
struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var verts = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    let xy = verts[vi];
    var out: VsOut;
    out.pos = vec4<f32>(xy, 0.0, 1.0);
    out.uv = vec2<f32>((xy.x + 1.0) * 0.5, 1.0 - (xy.y + 1.0) * 0.5);
    return out;
}

@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
// The input to the current modifier's pass group (same as `tex` for the first
// pass). Multi-pass modifiers read it to combine the original with an
// intermediate (e.g. lighting reads the source plus its blurred illumination).
@group(0) @binding(3) var src_tex: texture_2d<f32>;

// HSL helpers mirroring src/color.rs exactly so GPU output matches the CPU path.
fn rgb_to_hsl(c: vec3<f32>) -> vec3<f32> {
    let mx = max(c.r, max(c.g, c.b));
    let mn = min(c.r, min(c.g, c.b));
    let l = (mx + mn) * 0.5;
    let d = mx - mn;
    if (d < 1e-6) {
        return vec3<f32>(0.0, 0.0, l);
    }
    var s: f32;
    if (l > 0.5) {
        s = d / (2.0 - mx - mn);
    } else {
        s = d / (mx + mn);
    }
    var h: f32;
    if (mx == c.r) {
        h = (c.g - c.b) / d + select(0.0, 6.0, c.g < c.b);
    } else if (mx == c.g) {
        h = (c.b - c.r) / d + 2.0;
    } else {
        h = (c.r - c.g) / d + 4.0;
    }
    return vec3<f32>(h / 6.0, s, l);
}

fn hue_channel(p: f32, q: f32, t_in: f32) -> f32 {
    var t = t_in;
    if (t < 0.0) { t += 1.0; }
    if (t > 1.0) { t -= 1.0; }
    if (t < 1.0 / 6.0) { return p + (q - p) * 6.0 * t; }
    if (t < 1.0 / 2.0) { return q; }
    if (t < 2.0 / 3.0) { return p + (q - p) * (2.0 / 3.0 - t) * 6.0; }
    return p;
}

// Bilinear sample of the wrapped (tiling) input, mirroring pixel::sample_wrap
// so geometry modifiers match the CPU path. `p` is in source-pixel space.
fn sample_wrap(p: vec2<f32>) -> vec4<f32> {
    let dim = vec2<f32>(textureDimensions(tex));
    let xf = p.x - floor(p.x / dim.x) * dim.x;
    let yf = p.y - floor(p.y / dim.y) * dim.y;
    let w = i32(dim.x);
    let h = i32(dim.y);
    let x0 = i32(floor(xf)) % w;
    let y0 = i32(floor(yf)) % h;
    let x1 = (x0 + 1) % w;
    let y1 = (y0 + 1) % h;
    let tx = xf - floor(xf);
    let ty = yf - floor(yf);
    let c00 = textureLoad(tex, vec2<i32>(x0, y0), 0);
    let c10 = textureLoad(tex, vec2<i32>(x1, y0), 0);
    let c01 = textureLoad(tex, vec2<i32>(x0, y1), 0);
    let c11 = textureLoad(tex, vec2<i32>(x1, y1), 0);
    let a = mix(c00, c10, tx);
    let b = mix(c01, c11, tx);
    return mix(a, b, ty);
}

fn hsl_to_rgb(hsl: vec3<f32>) -> vec3<f32> {
    let h = hsl.x;
    let s = hsl.y;
    let l = hsl.z;
    if (s <= 0.0) {
        return vec3<f32>(l, l, l);
    }
    var q: f32;
    if (l < 0.5) {
        q = l * (1.0 + s);
    } else {
        q = l + s - l * s;
    }
    let p = 2.0 * l - q;
    return vec3<f32>(
        hue_channel(p, q, h + 1.0 / 3.0),
        hue_channel(p, q, h),
        hue_channel(p, q, h - 1.0 / 3.0),
    );
}
"#;

/// How a pass's output size relates to its input size.
#[derive(Clone, Copy)]
pub enum OutSize {
    /// Same dimensions as the input.
    Same,
    /// Width and height swapped (90° rotation).
    Swap,
    /// Half the input size (rounded up, min 1) - for downsample pyramids.
    Half,
    /// The group's input size (e.g. a final pass that upsamples a small level
    /// back to full resolution).
    Source,
    /// Fixed dimensions.
    Fixed(u32, u32),
}

/// One fragment-shader pass in a modifier's GPU chain.
pub struct GpuPass {
    /// Stable cache key for the compiled pipeline (unique per shader).
    pub key: &'static str,
    /// Fragment stage (and any `@group(0) @binding(2)` uniform); the shared
    /// vertex header is prepended automatically.
    pub fragment: String,
    /// Uniform buffer bytes for binding 2 (empty when the shader has none).
    pub uniforms: Vec<u8>,
    /// Output size relative to the input.
    pub out_size: OutSize,
}

impl GpuPass {
    pub fn new(key: &'static str, fragment: impl Into<String>) -> Self {
        Self {
            key,
            fragment: fragment.into(),
            uniforms: Vec::new(),
            out_size: OutSize::Same,
        }
    }

    pub fn with_uniforms(mut self, bytes: &[u8]) -> Self {
        self.uniforms = bytes.to_vec();
        self
    }

    pub fn with_out_size(mut self, out_size: OutSize) -> Self {
        self.out_size = out_size;
        self
    }
}

/// Pack `f32` params into uniform-buffer bytes, padding to a multiple of four
/// so each group lands on a 16-byte (vec4) boundary as WGSL `uniform` requires.
/// Shaders read them as `array<vec4<f32>, N>` (see binding 2).
pub fn uniforms(values: &[f32]) -> Vec<u8> {
    let mut padded = values.to_vec();
    while padded.len() % 4 != 0 {
        padded.push(0.0);
    }
    bytemuck::cast_slice(&padded).to_vec()
}

pub struct GpuContext {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    sampler: wgpu::Sampler,
    layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    pipelines: Mutex<HashMap<&'static str, Arc<wgpu::RenderPipeline>>>,
}

impl GpuContext {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("kifla.sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kifla.bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("kifla.pipeline_layout"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });

        Self {
            device,
            queue,
            sampler,
            layout,
            pipeline_layout,
            pipelines: Mutex::new(HashMap::new()),
        }
    }

    fn pipeline(&self, pass: &GpuPass) -> Arc<wgpu::RenderPipeline> {
        if let Some(p) = self.pipelines.lock().unwrap().get(pass.key) {
            return p.clone();
        }
        let source = format!("{SHADER_HEADER}\n{}", pass.fragment);
        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(pass.key),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(pass.key),
                layout: Some(&self.pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: "vs_main",
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: TEX_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            });
        let pipeline = Arc::new(pipeline);
        self.pipelines
            .lock()
            .unwrap()
            .insert(pass.key, pipeline.clone());
        pipeline
    }

    fn make_texture(&self, w: u32, h: u32) -> wgpu::Texture {
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("kifla.tex"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TEX_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    }

    /// Run the modifier chain (one `Vec<GpuPass>` group per enabled modifier) on
    /// `input` and read the result back to an image. Within a group, each pass's
    /// input is bound at binding 0 and the group's first input at binding 3, so
    /// multi-pass modifiers can combine the original with intermediates.
    pub fn apply(&self, input: &image::RgbaImage, groups: &[Vec<GpuPass>]) -> image::RgbaImage {
        use wgpu::util::DeviceExt;

        // All textures live in a pool so views stay valid until submission; we
        // index into it rather than moving textures around.
        let mut pool: Vec<wgpu::Texture> = Vec::new();
        let mut dims: Vec<(u32, u32)> = Vec::new();
        let (iw, ih) = (input.width(), input.height());
        pool.push(self.make_texture(iw, ih));
        dims.push((iw, ih));
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &pool[0],
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            input,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(iw * 4),
                rows_per_image: Some(ih),
            },
            wgpu::Extent3d {
                width: iw,
                height: ih,
                depth_or_array_layers: 1,
            },
        );

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("kifla.enc") });

        let mut cur = 0usize;
        for group in groups {
            let group_in = cur;
            for pass in group {
                let (w, h) = dims[cur];
                let (ow, oh) = match pass.out_size {
                    OutSize::Same => (w, h),
                    OutSize::Swap => (h, w),
                    OutSize::Half => (w.div_ceil(2).max(1), h.div_ceil(2).max(1)),
                    OutSize::Source => dims[group_in],
                    OutSize::Fixed(fw, fh) => (fw.max(1), fh.max(1)),
                };
                pool.push(self.make_texture(ow, oh));
                dims.push((ow, oh));
                let tgt = pool.len() - 1;
                let pipeline = self.pipeline(pass);

                let bytes: &[u8] = if pass.uniforms.is_empty() {
                    &[0u8; 16]
                } else {
                    &pass.uniforms
                };
                let ubuf =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("kifla.uniforms"),
                            contents: bytes,
                            usage: wgpu::BufferUsages::UNIFORM,
                        });

                let in_view = pool[cur].create_view(&wgpu::TextureViewDescriptor::default());
                let src_view = pool[group_in].create_view(&wgpu::TextureViewDescriptor::default());
                let out_view = pool[tgt].create_view(&wgpu::TextureViewDescriptor::default());
                let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("kifla.bind"),
                    layout: &self.layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&in_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: ubuf.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(&src_view),
                        },
                    ],
                });

                {
                    let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("kifla.pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &out_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    rp.set_pipeline(&pipeline);
                    rp.set_bind_group(0, &bind, &[]);
                    rp.draw(0..3, 0..1);
                }

                cur = tgt;
            }
        }

        let current = &pool[cur];
        let (w, h) = dims[cur];

        // Copy the final texture into a read-back buffer (rows padded to 256).
        let padded = (w * 4).div_ceil(256) * 256;
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("kifla.readback"),
            size: (padded * h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: current,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &readback,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(Some(encoder.finish()));

        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device.poll(wgpu::Maintain::Wait);

        let mapped = slice.get_mapped_range();
        let mut out = image::RgbaImage::new(w, h);
        let row = (w * 4) as usize;
        for y in 0..h as usize {
            let src = y * padded as usize;
            out.as_mut()[y * row..(y + 1) * row].copy_from_slice(&mapped[src..src + row]);
        }
        out
    }
}
