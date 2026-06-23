use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use eframe::wgpu;

const TEX_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
// Display textures are sRGB so egui samples them the same way it does its own
// managed textures (sampling decodes sRGB->linear). The processing pipeline
// stays plain `Rgba8Unorm` so modifier math runs directly on the stored values.
const DISPLAY_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

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

// HSL helpers shared by the color modifiers.
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

// Bilinear sample of the wrapped (tiling) input, used by the geometry
// modifiers. `p` is in source-pixel space.
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

// Fullscreen-triangle downsample used for mip generation: bilinearly sampling
// the previous (full-size) level at the half-size target's centers averages
// each 2x2 block.
const MIP_SHADER: &str = r#"
struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var verts = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    let xy = verts[vi];
    var out: VsOut;
    out.pos = vec4<f32>(xy, 0.0, 1.0);
    out.uv = vec2<f32>((xy.x + 1.0) * 0.5, 1.0 - (xy.y + 1.0) * 0.5);
    return out;
}
@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return textureSampleLevel(tex, samp, in.uv, 0.0);
}
"#;

/// One modifier's GPU work: either a sequence of fragment passes, or the
/// indexed-color compute step (k-means palette + nearest match + dither).
pub enum GpuStep {
    Fragment(Vec<GpuPass>),
    IndexColor {
        colors: u32,
        dither: bool,
        amount: f32,
    },
}

/// Pack `f32` params into uniform-buffer bytes, padding to a multiple of four
/// so each group lands on a 16-byte (vec4) boundary as WGSL `uniform` requires.
/// Shaders read them as `array<vec4<f32>, N>` (see binding 2).
pub fn uniforms(values: &[f32]) -> Vec<u8> {
    let mut padded = values.to_vec();
    padded.resize(values.len().next_multiple_of(4), 0.0);
    bytemuck::cast_slice(&padded).to_vec()
}

pub struct GpuContext {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    sampler: wgpu::Sampler,
    layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    pipelines: Mutex<HashMap<&'static str, Arc<wgpu::RenderPipeline>>>,
    kmeans: Mutex<Option<Arc<Kmeans>>>,
    mip_layout: wgpu::BindGroupLayout,
    mip_pipeline: wgpu::RenderPipeline,
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

        // Pipeline for generating mip levels (downsample previous level).
        let mip_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kifla.mip_bgl"),
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
            ],
        });
        let mip_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("kifla.mip_pl"),
            bind_group_layouts: &[&mip_layout],
            push_constant_ranges: &[],
        });
        let mip_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("kifla.mip"),
            source: wgpu::ShaderSource::Wgsl(MIP_SHADER.into()),
        });
        let mip_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("kifla.mip_pipeline"),
            layout: Some(&mip_pl),
            vertex: wgpu::VertexState {
                module: &mip_module,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &mip_module,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: DISPLAY_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Self {
            device,
            queue,
            sampler,
            layout,
            pipeline_layout,
            pipelines: Mutex::new(HashMap::new()),
            kmeans: Mutex::new(None),
            mip_layout,
            mip_pipeline,
        }
    }

    /// Upload an image into a texture with a full mip chain (mip 0 written
    /// directly, lower levels generated by repeated 2x box downsampling). The
    /// returned texture is for display - sample it with a mipmapping sampler so
    /// zoomed-out previews are smooth instead of aliased.
    pub fn upload_mipmapped(&self, img: &image::RgbaImage) -> wgpu::Texture {
        let (w, h) = (img.width(), img.height());
        let levels = (32 - w.max(h).max(1).leading_zeros()).max(1);
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("kifla.display"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DISPLAY_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            img,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("kifla.mipgen") });
        let view = |level: u32| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                base_mip_level: level,
                mip_level_count: Some(1),
                ..Default::default()
            })
        };
        for level in 1..levels {
            let src = view(level - 1);
            let dst = view(level);
            let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("kifla.mip_bind"),
                layout: &self.mip_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&src),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("kifla.mip_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &dst,
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
            rp.set_pipeline(&self.mip_pipeline);
            rp.set_bind_group(0, &bind, &[]);
            rp.draw(0..3, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
        texture
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
                | wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    }

    /// Run the modifier chain (one [`GpuStep`] per enabled modifier) on `input`
    /// and read the result back to an image. Fragment groups are batched into one
    /// encoder; an Indexed Color step flushes the encoder (its compute submits
    /// must see prior results), runs, then a fresh encoder resumes.
    pub fn apply(&self, input: &image::RgbaImage, steps: &[GpuStep]) -> image::RgbaImage {
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

        let mut encoder: Option<wgpu::CommandEncoder> = None;
        let mut cur = 0usize;
        for step in steps {
            match step {
                GpuStep::Fragment(group) => {
                    let enc = encoder.get_or_insert_with(|| {
                        self.device.create_command_encoder(
                            &wgpu::CommandEncoderDescriptor { label: Some("kifla.enc") },
                        )
                    });
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
                        let ubuf = self.device.create_buffer_init(
                            &wgpu::util::BufferInitDescriptor {
                                label: Some("kifla.uniforms"),
                                contents: bytes,
                                usage: wgpu::BufferUsages::UNIFORM,
                            },
                        );

                        let in_view =
                            pool[cur].create_view(&wgpu::TextureViewDescriptor::default());
                        let src_view =
                            pool[group_in].create_view(&wgpu::TextureViewDescriptor::default());
                        let out_view =
                            pool[tgt].create_view(&wgpu::TextureViewDescriptor::default());
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
                            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("kifla.pass"),
                                color_attachments: &[Some(
                                    wgpu::RenderPassColorAttachment {
                                        view: &out_view,
                                        resolve_target: None,
                                        ops: wgpu::Operations {
                                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                                            store: wgpu::StoreOp::Store,
                                        },
                                    },
                                )],
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
                GpuStep::IndexColor {
                    colors,
                    dither,
                    amount,
                } => {
                    // Flush queued fragment work so the compute step reads it.
                    if let Some(enc) = encoder.take() {
                        self.queue.submit(Some(enc.finish()));
                    }
                    let (w, h) = dims[cur];
                    let out = self.index_color(&pool[cur], w, h, *colors, *dither, *amount);
                    pool.push(out);
                    dims.push((w, h));
                    cur = pool.len() - 1;
                }
            }
        }

        let mut encoder = encoder.unwrap_or_else(|| {
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("kifla.enc") })
        });
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

    fn kmeans(&self) -> Arc<Kmeans> {
        let mut guard = self.kmeans.lock().unwrap();
        guard
            .get_or_insert_with(|| Arc::new(Kmeans::new(&self.device, &self.queue)))
            .clone()
    }

    /// Quantize `input` to `colors` adaptive colors via GPU k-means (a few
    /// iterations), then map each pixel to its nearest palette entry, optionally
    /// with ordered (Bayer) dithering. Returns the quantized texture.
    fn index_color(
        &self,
        input: &wgpu::Texture,
        w: u32,
        h: u32,
        colors: u32,
        dither: bool,
        amount: f32,
    ) -> wgpu::Texture {
        use wgpu::util::DeviceExt;

        let n = colors.clamp(2, 256);
        let npix = w * h;
        let km = self.kmeans();
        let output = self.make_texture(w, h);

        let centroids = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("kmeans.centroids"),
            size: 256 * 16,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let accum = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("kmeans.accum"),
            size: 256 * 4 * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let in_view = input.create_view(&wgpu::TextureViewDescriptor::default());
        let out_view = output.create_view(&wgpu::TextureViewDescriptor::default());
        let noise_view = km.blue_noise.create_view(&wgpu::TextureViewDescriptor::default());

        let params = |dither: u32| -> wgpu::Buffer {
            // std140-ish: u32 n, npix, w, dither, f32 amount, + pad to 16-byte.
            let mut bytes = Vec::new();
            bytes.extend_from_slice(&n.to_le_bytes());
            bytes.extend_from_slice(&npix.to_le_bytes());
            bytes.extend_from_slice(&w.to_le_bytes());
            bytes.extend_from_slice(&dither.to_le_bytes());
            bytes.extend_from_slice(&amount.to_le_bytes());
            bytes.extend_from_slice(&[0u8; 12]);
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("kmeans.params"),
                    contents: &bytes,
                    usage: wgpu::BufferUsages::UNIFORM,
                })
        };

        let make_bind = |params: &wgpu::Buffer| {
            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("kmeans.bind"),
                layout: &km.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&in_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: centroids.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: accum.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: params.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&out_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(&noise_view),
                    },
                ],
            })
        };

        // Per-pixel passes need one workgroup per 256 pixels, but a single
        // dispatch dimension is capped at 65535 workgroups - so spread them over
        // a 2D grid and reconstruct the linear pixel index in the shader.
        let total_wg = npix.div_ceil(256);
        let grid_x = total_wg.min(65535);
        let grid_y = total_wg.div_ceil(grid_x.max(1));
        let groups_n = n.div_ceil(256).max(1);

        // Init centroids from spread-out source pixels.
        let p_init = params(0);
        let bind_init = make_bind(&p_init);
        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("kmeans.init") });
        {
            let mut cp = enc.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            cp.set_pipeline(&km.init);
            cp.set_bind_group(0, &bind_init, &[]);
            cp.dispatch_workgroups(groups_n, 1, 1);
        }
        self.queue.submit(Some(enc.finish()));

        // Lloyd iterations: clear accumulators, assign+accumulate, recompute.
        for _ in 0..8 {
            self.queue.write_buffer(&accum, 0, &[0u8; 256 * 4 * 4]);
            let p = params(0);
            let bind = make_bind(&p);
            let mut enc = self.device.create_command_encoder(
                &wgpu::CommandEncoderDescriptor { label: Some("kmeans.iter") },
            );
            {
                let mut cp = enc.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                cp.set_pipeline(&km.assign);
                cp.set_bind_group(0, &bind, &[]);
                cp.dispatch_workgroups(grid_x, grid_y, 1);
                cp.set_pipeline(&km.update);
                cp.set_bind_group(0, &bind, &[]);
                cp.dispatch_workgroups(groups_n, 1, 1);
            }
            self.queue.submit(Some(enc.finish()));
        }

        // Final map (+ optional Bayer dither) into the output texture.
        let p_map = params(if dither { 1 } else { 0 });
        let bind_map = make_bind(&p_map);
        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("kmeans.map") });
        {
            let mut cp = enc.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            cp.set_pipeline(&km.map);
            cp.set_bind_group(0, &bind_map, &[]);
            cp.dispatch_workgroups(grid_x, grid_y, 1);
        }
        self.queue.submit(Some(enc.finish()));

        output
    }
}

/// Compute pipelines for GPU k-means colour quantization.
struct Kmeans {
    layout: wgpu::BindGroupLayout,
    init: wgpu::ComputePipeline,
    assign: wgpu::ComputePipeline,
    update: wgpu::ComputePipeline,
    map: wgpu::ComputePipeline,
    blue_noise: wgpu::Texture,
}

const BLUE_NOISE_SIZE: u32 = 64;

impl Kmeans {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        // Precompute a blue-noise tile for high-quality ordered dithering.
        let tile = crate::bluenoise::generate(BLUE_NOISE_SIZE as usize);
        let blue_noise = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("kmeans.bluenoise"),
            size: wgpu::Extent3d {
                width: BLUE_NOISE_SIZE,
                height: BLUE_NOISE_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &blue_noise,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &tile,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(BLUE_NOISE_SIZE),
                rows_per_image: Some(BLUE_NOISE_SIZE),
            },
            wgpu::Extent3d {
                width: BLUE_NOISE_SIZE,
                height: BLUE_NOISE_SIZE,
                depth_or_array_layers: 1,
            },
        );
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kmeans.bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: TEX_FORMAT,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("kmeans.pl"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("kmeans"),
            source: wgpu::ShaderSource::Wgsl(KMEANS_SHADER.into()),
        });
        let mk = |entry: &str| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&pl),
                module: &module,
                entry_point: entry,
            })
        };
        Self {
            init: mk("init"),
            assign: mk("assign"),
            update: mk("update"),
            map: mk("map"),
            layout,
            blue_noise,
        }
    }
}

const KMEANS_SHADER: &str = r#"
@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var<storage, read_write> centroids: array<vec4<f32>, 256>;
@group(0) @binding(2) var<storage, read_write> accum: array<atomic<u32>, 1024>;
struct Params { n: u32, npix: u32, w: u32, dither: u32, amount: f32 };
@group(0) @binding(3) var<uniform> params: Params;
@group(0) @binding(4) var out_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(5) var bluenoise: texture_2d<f32>;

fn coord_of(i: u32) -> vec2<i32> {
    return vec2<i32>(i32(i % params.w), i32(i / params.w));
}

fn nearest(c: vec3<f32>) -> u32 {
    var best = 0u;
    var best_d = 1e9;
    for (var i = 0u; i < params.n; i = i + 1u) {
        let d = distance(c, centroids[i].rgb);
        if (d < best_d) { best_d = d; best = i; }
    }
    return best;
}

@compute @workgroup_size(256)
fn init(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= params.n) { return; }
    // Spread initial centroids across the image.
    let pidx = (i * params.npix) / params.n;
    let c = textureLoad(tex, coord_of(pidx), 0);
    centroids[i] = vec4<f32>(c.rgb, 1.0);
}

@compute @workgroup_size(256)
fn assign(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(num_workgroups) nwg: vec3<u32>,
) {
    let i = gid.y * (nwg.x * 256u) + gid.x;
    if (i >= params.npix) { return; }
    let c = textureLoad(tex, coord_of(i), 0).rgb;
    let k = nearest(c);
    atomicAdd(&accum[k * 4u + 0u], u32(round(c.r * 255.0)));
    atomicAdd(&accum[k * 4u + 1u], u32(round(c.g * 255.0)));
    atomicAdd(&accum[k * 4u + 2u], u32(round(c.b * 255.0)));
    atomicAdd(&accum[k * 4u + 3u], 1u);
}

@compute @workgroup_size(256)
fn update(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= params.n) { return; }
    let cnt = atomicLoad(&accum[i * 4u + 3u]);
    if (cnt > 0u) {
        let r = f32(atomicLoad(&accum[i * 4u + 0u])) / f32(cnt) / 255.0;
        let g = f32(atomicLoad(&accum[i * 4u + 1u])) / f32(cnt) / 255.0;
        let b = f32(atomicLoad(&accum[i * 4u + 2u])) / f32(cnt) / 255.0;
        centroids[i] = vec4<f32>(r, g, b, 1.0);
    }
}

@compute @workgroup_size(256)
fn map(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(num_workgroups) nwg: vec3<u32>,
) {
    let i = gid.y * (nwg.x * 256u) + gid.x;
    if (i >= params.npix) { return; }
    let coord = coord_of(i);
    let src = textureLoad(tex, coord, 0);
    var c = src.rgb;
    if (params.dither == 1u) {
        // Tiled blue-noise threshold (organic, grid-free, well dispersed).
        let bn = textureLoad(bluenoise, vec2<i32>(coord.x & 63, coord.y & 63), 0).r;
        let t = bn - 0.5;
        c = clamp(c + vec3<f32>(t * params.amount * 0.5), vec3<f32>(0.0), vec3<f32>(1.0));
    }
    let k = nearest(c);
    textureStore(out_tex, coord, vec4<f32>(centroids[k].rgb, src.a));
}
"#;
