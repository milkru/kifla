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
"#;

/// One fragment-shader pass in a modifier's GPU chain.
pub struct GpuPass {
    /// Stable cache key for the compiled pipeline (unique per shader).
    pub key: &'static str,
    /// Fragment stage (and any `@group(0) @binding(2)` uniform); the shared
    /// vertex header is prepended automatically.
    pub fragment: String,
    /// Uniform buffer bytes for binding 2 (empty when the shader has none).
    pub uniforms: Vec<u8>,
    /// Output size; `None` keeps the current size.
    pub out_size: Option<(u32, u32)>,
}

impl GpuPass {
    pub fn new(key: &'static str, fragment: impl Into<String>) -> Self {
        Self {
            key,
            fragment: fragment.into(),
            uniforms: Vec::new(),
            out_size: None,
        }
    }

    pub fn with_uniforms(mut self, bytes: &[u8]) -> Self {
        self.uniforms = bytes.to_vec();
        self
    }

    pub fn with_out_size(mut self, w: u32, h: u32) -> Self {
        self.out_size = Some((w, h));
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

    /// Run the modifier chain on `input` and read the result back to an image.
    pub fn apply(&self, input: &image::RgbaImage, passes: &[GpuPass]) -> image::RgbaImage {
        use wgpu::util::DeviceExt;

        let (mut w, mut h) = (input.width(), input.height());
        let mut current = self.make_texture(w, h);
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &current,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            input,
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
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("kifla.enc") });

        // Keep resources alive until the queue submission below.
        let mut keep: Vec<wgpu::Texture> = Vec::new();

        for pass in passes {
            let (ow, oh) = pass.out_size.unwrap_or((w, h));
            let target = self.make_texture(ow, oh);
            let pipeline = self.pipeline(pass);

            let bytes: &[u8] = if pass.uniforms.is_empty() {
                &[0u8; 16]
            } else {
                &pass.uniforms
            };
            let ubuf = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("kifla.uniforms"),
                    contents: bytes,
                    usage: wgpu::BufferUsages::UNIFORM,
                });

            let in_view = current.create_view(&wgpu::TextureViewDescriptor::default());
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
                ],
            });

            let out_view = target.create_view(&wgpu::TextureViewDescriptor::default());
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

            keep.push(current);
            current = target;
            w = ow;
            h = oh;
        }

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
                texture: &current,
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
