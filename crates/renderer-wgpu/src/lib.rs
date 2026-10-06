mod failure;
mod geometry;

use failure::Failure;
use geometry::{Geometry, VERTEX_BYTES, Viewport};
use platform_api::render::{
    Frame, Image, MAX_QUADS, MAX_TEXTURE_BYTES, MAX_TEXTURES, Sampling, TextureUpload,
};
use std::fmt;
use wgpu::util::DeviceExt;

const GPU_QUADS: usize = MAX_QUADS + 1;
const FRAMES_IN_FLIGHT: usize = 2;

#[derive(Debug)]
pub struct RendererError(String);

impl fmt::Display for RendererError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for RendererError {}

fn error(value: impl fmt::Display) -> RendererError {
    RendererError(value.to_string())
}

#[derive(Debug)]
pub struct FrameStats {
    pub commands: usize,
    pub quads: usize,
    pub batches: usize,
    pub vertex_bytes: usize,
}

struct Texture {
    groups: [wgpu::BindGroup; 2],
    width: u16,
    height: u16,
}

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    drawable: bool,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    samplers: [wgpu::Sampler; 2],
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    textures: [Option<Texture>; MAX_TEXTURES],
    white: Texture,
    texture_bytes: usize,
    geometry: Geometry,
    failure: Failure,
    pending: [Option<wgpu::SubmissionIndex>; FRAMES_IN_FLIGHT],
    next_submission: usize,
}

impl Renderer {
    pub async fn new(
        target: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
    ) -> Result<Self, RendererError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance.create_surface(target).map_err(error)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                power_preference: wgpu::PowerPreference::LowPower,
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .map_err(error)?;
        eprintln!("wgpu adapter: {:?}", adapter.get_info());
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .map_err(error)?;
        let failure = Failure::new(&device);
        let mut config = surface
            .get_default_config(&adapter, width.max(1), height.max(1))
            .ok_or_else(|| error("surface unsupported"))?;
        config.format = surface
            .get_capabilities(&adapter)
            .formats
            .into_iter()
            .find(wgpu::TextureFormat::is_srgb)
            .ok_or_else(|| error("sRGB surface unavailable"))?;
        config.present_mode = wgpu::PresentMode::Fifo;
        config.desired_maximum_frame_latency = u32::try_from(FRAMES_IN_FLIGHT).unwrap();
        let pipeline = make_pipeline(&device, config.format);
        failure.check()?;
        let layout = pipeline.get_bind_group_layout(0);
        let samplers = [wgpu::FilterMode::Nearest, wgpu::FilterMode::Linear].map(|filter| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                mag_filter: filter,
                min_filter: filter,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                ..Default::default()
            })
        });
        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vertices"),
            size: u64::try_from(GPU_QUADS * 4 * VERTEX_BYTES).unwrap(),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut index_bytes = Vec::new();
        index_bytes
            .try_reserve_exact(GPU_QUADS * 6 * 4)
            .map_err(error)?;
        for quad in 0..u32::try_from(GPU_QUADS).unwrap() {
            for offset in [0, 1, 2, 0, 2, 3] {
                index_bytes.extend_from_slice(&(quad * 4 + offset).to_ne_bytes());
            }
        }
        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("indices"),
            contents: &index_bytes,
            usage: wgpu::BufferUsages::INDEX,
        });
        let white = make_texture(
            &device,
            &queue,
            &layout,
            &samplers,
            Image::new(1, 1, &[255; 4]).unwrap(),
            &failure,
        )?;
        failure.check()?;
        let mut renderer = Self {
            surface,
            device,
            queue,
            config,
            drawable: false,
            pipeline,
            layout,
            samplers,
            vertices,
            indices,
            textures: std::array::from_fn(|_| None),
            white,
            texture_bytes: 0,
            geometry: Geometry::new()?,
            failure,
            pending: std::array::from_fn(|_| None),
            next_submission: 0,
        };
        renderer.resize(width, height)?;
        Ok(renderer)
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), RendererError> {
        self.failure.wait(&self.device, None)?;
        self.pending = std::array::from_fn(|_| None);
        let limit = self.device.limits().max_texture_dimension_2d;
        if width > limit || height > limit {
            return Err(error("surface dimensions exceed device limits"));
        }
        self.drawable = width != 0 && height != 0;
        if self.drawable {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
        self.failure.check()
    }

    pub fn upload_texture(&mut self, upload: TextureUpload<'_>) -> Result<(), RendererError> {
        self.failure.wait(&self.device, None)?;
        let slot = upload.id.index();
        assert!(self.textures[slot].is_none(), "texture already uploaded");
        let total = self.texture_bytes + upload.image.rgba().len();
        if total > MAX_TEXTURE_BYTES {
            return Err(error("texture budget exhausted"));
        }
        let texture = make_texture(
            &self.device,
            &self.queue,
            &self.layout,
            &self.samplers,
            upload.image,
            &self.failure,
        )?;
        self.failure.check()?;
        self.queue.submit([]);
        self.failure.wait(&self.device, None)?;
        self.textures[slot] = Some(texture);
        self.texture_bytes = total;
        Ok(())
    }

    pub fn draw(&mut self, frame: &Frame<'_>) -> Result<Option<FrameStats>, RendererError> {
        self.failure.check()?;
        if let Some(submission) = self.pending[self.next_submission].take() {
            self.failure.wait(&self.device, Some(submission))?;
        }
        if !self.drawable {
            return Ok(None);
        }
        let Some(viewport) = Viewport::new(frame.canvas, self.config.width, self.config.height)
        else {
            return Ok(None);
        };
        self.geometry.build(frame, viewport, &self.textures);
        let (output, suboptimal) = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(output) => (output, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(output) => (output, true),
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(None);
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(None);
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                return Err(error("surface lost; recreate renderer"));
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(error("surface validation failed"));
            }
        };
        self.queue
            .write_buffer(&self.vertices, 0, &self.geometry.bytes);
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_vertex_buffer(0, self.vertices.slice(..));
            pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.set_viewport(
                viewport.x,
                viewport.y,
                viewport.width,
                viewport.height,
                0.0,
                1.0,
            );
            for batch in &self.geometry.batches {
                let texture = batch
                    .texture
                    .map_or(&self.white, |slot| self.textures[slot].as_ref().unwrap());
                let sampler = usize::from(batch.sampling == Sampling::Linear);
                pass.set_bind_group(0, &texture.groups[sampler], &[]);
                let [x, y, width, height] = batch.clip;
                pass.set_scissor_rect(x, y, width, height);
                pass.draw_indexed(batch.start * 6..batch.end * 6, 0, 0..1);
            }
        }
        self.pending[self.next_submission] = Some(self.queue.submit([encoder.finish()]));
        self.next_submission = (self.next_submission + 1) % FRAMES_IN_FLIGHT;
        self.failure.check()?;
        self.queue.present(output);
        if suboptimal {
            drop(view);
            self.resize(self.config.width, self.config.height)?;
        }
        Ok(Some(FrameStats {
            commands: frame.commands.len(),
            quads: (self.geometry.bytes.len() / (4 * VERTEX_BYTES)).saturating_sub(1),
            batches: self.geometry.batches.len(),
            vertex_bytes: self.geometry.bytes.len(),
        }))
    }
}

fn make_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    samplers: &[wgpu::Sampler; 2],
    image: Image<'_>,
    failure: &Failure,
) -> Result<Texture, RendererError> {
    let size = wgpu::Extent3d {
        width: u32::from(image.width()),
        height: u32::from(image.height()),
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("image"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    failure.check()?;
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        image.rgba(),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(size.width * 4),
            rows_per_image: Some(size.height),
        },
        size,
    );
    failure.check()?;
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let groups = samplers.each_ref().map(|sampler| {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("image"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    });
    failure.check()?;
    Ok(Texture {
        groups,
        width: image.width(),
        height: image.height(),
    })
}

fn make_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("quad"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
    });
    let attributes =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4, 3 => Float32x4];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("quads"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: u64::try_from(VERTEX_BYTES).unwrap(),
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &attributes,
            })],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fragment"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}
