mod failure;
mod geometry;
mod meshes;
mod validation;

use failure::Failure;
use geometry::{Geometry, VERTEX_BYTES, Viewport};
use meshes::Meshes;
use platform_api::render::{Frame, Sampling, TextureError, TextureId, TextureUpload};
use std::fmt;
use wgpu::util::DeviceExt;

const FRAMES_IN_FLIGHT: usize = 2;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub commands: usize,
    pub quads: usize,
    pub mesh_vertices: usize,
    pub mesh_indices: usize,
    pub textures: usize,
    pub texture_side: u32,
    pub texture_bytes: usize,
}

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
    width: u32,
    height: u32,
    bytes: usize,
}

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    depth: Option<wgpu::TextureView>,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    samplers: [wgpu::Sampler; 2],
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    textures: Vec<Option<Texture>>,
    limits: Limits,
    white: Texture,
    texture_bytes: usize,
    geometry: Geometry,
    meshes: Meshes,
    failure: Failure,
    pending: [Option<wgpu::SubmissionIndex>; FRAMES_IN_FLIGHT],
    upload_pending: Option<wgpu::SubmissionIndex>,
    next_submission: usize,
}

impl Renderer {
    pub async fn new(
        target: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
        limits: Limits,
    ) -> Result<Self, RendererError> {
        let gpu_quads = limits
            .quads
            .checked_add(1)
            .ok_or_else(|| error("quad limit overflow"))?;
        let vertex_bytes = gpu_quads
            .checked_mul(4 * VERTEX_BYTES)
            .ok_or_else(|| error("vertex limit overflow"))?;
        let quad_count = u32::try_from(gpu_quads).map_err(error)?;
        if quad_count > u32::MAX / 6 || limits.textures > usize::from(u16::MAX) + 1 {
            return Err(error("renderer limits exceed index ranges"));
        }
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
        let buffer_limit = device.limits().max_buffer_size;
        if u64::try_from(vertex_bytes).map_err(error)? > buffer_limit {
            return Err(error("renderer buffers exceed device limits"));
        }
        let config = surface_config(&surface, &adapter, width, height)?;
        let pipeline = make_pipeline(&device, config.format.add_srgb_suffix());
        let meshes = Meshes::new(&device, config.format.add_srgb_suffix(), limits)?;
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
            size: u64::try_from(vertex_bytes).map_err(error)?,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let indices = make_indices(&device, quad_count)?;
        let white = make_texture(
            &device,
            &queue,
            &layout,
            &samplers,
            TextureUpload {
                id: TextureId(0),
                width: 1,
                height: 1,
                rgba: &[255; 4],
            },
            &failure,
        )?;
        failure.check()?;
        let mut textures = Vec::new();
        textures.try_reserve_exact(limits.textures).map_err(error)?;
        textures.resize_with(limits.textures, || None);
        let mut renderer = Self {
            surface,
            device,
            queue,
            config,
            depth: None,
            pipeline,
            layout,
            samplers,
            vertices,
            indices,
            textures,
            limits,
            white,
            texture_bytes: 0,
            geometry: Geometry::new(gpu_quads)?,
            meshes,
            failure,
            pending: std::array::from_fn(|_| None),
            upload_pending: None,
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
        self.depth = if width != 0 && height != 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("scene depth"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            Some(texture.create_view(&wgpu::TextureViewDescriptor::default()))
        } else {
            None
        };
        self.failure.check()
    }

    pub fn upload_texture(&mut self, upload: TextureUpload<'_>) -> Result<(), TextureError> {
        let slot = usize::from(upload.id.0);
        let previous = self.textures.get(slot).ok_or(TextureError::InvalidId)?;
        validation::image(upload)?;
        if upload.width > self.limits.texture_side
            || upload.height > self.limits.texture_side
            || upload.width > self.device.limits().max_texture_dimension_2d
            || upload.height > self.device.limits().max_texture_dimension_2d
        {
            return Err(TextureError::CapacityExceeded);
        }
        let previous_bytes = previous.as_ref().map_or(0, |texture| texture.bytes);
        let total = (self.texture_bytes - previous_bytes)
            .checked_add(upload.rgba.len())
            .filter(|&total| total <= self.limits.texture_bytes)
            .ok_or(TextureError::CapacityExceeded)?;
        self.failure
            .check()
            .map_err(|_| TextureError::Unavailable)?;
        if let Some(submission) = self.upload_pending.take() {
            self.failure
                .wait(&self.device, Some(submission))
                .map_err(|_| TextureError::Unavailable)?;
        }
        let texture = make_texture(
            &self.device,
            &self.queue,
            &self.layout,
            &self.samplers,
            upload,
            &self.failure,
        )
        .map_err(|_| TextureError::Unavailable)?;
        self.upload_pending = Some(self.queue.submit([]));
        self.failure
            .check()
            .map_err(|_| TextureError::Unavailable)?;
        self.textures[slot] = Some(texture);
        self.texture_bytes = total;
        Ok(())
    }

    pub fn release_texture(&mut self, id: TextureId) -> Result<(), TextureError> {
        let slot = usize::from(id.0);
        let texture = self
            .textures
            .get(slot)
            .and_then(Option::as_ref)
            .ok_or(TextureError::InvalidId)?;
        let bytes = texture.bytes;
        self.failure
            .wait(&self.device, None)
            .map_err(|_| TextureError::Unavailable)?;
        self.textures[slot] = None;
        self.texture_bytes -= bytes;
        self.upload_pending = None;
        self.pending = std::array::from_fn(|_| None);
        Ok(())
    }

    pub fn draw(&mut self, frame: &Frame<'_>) -> Result<Option<FrameStats>, RendererError> {
        validation::frame(frame, self.limits, &self.textures)?;
        self.meshes.prepare(frame)?;
        self.failure.check()?;
        if let Some(submission) = self.pending[self.next_submission].take() {
            self.failure.wait(&self.device, Some(submission))?;
        }
        let Some(depth) = &self.depth else {
            return Ok(None);
        };
        let Some(viewport) =
            Viewport::new(frame.canvas_size, self.config.width, self.config.height)
        else {
            return Ok(None);
        };
        self.geometry.build(frame, viewport, &self.textures)?;
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
        self.meshes.upload(&self.queue);
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.config.format.add_srgb_suffix()),
            ..Default::default()
        });
        let commands = self.encode_frame(&view, depth, viewport);
        self.pending[self.next_submission] = Some(self.queue.submit([commands]));
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
            batches: 1
                + self.meshes.draw_count()
                + self
                    .geometry
                    .batches
                    .iter()
                    .filter(|batch| batch.end > 1)
                    .count(),
            vertex_bytes: self.geometry.bytes.len() + self.meshes.vertex_bytes(),
        }))
    }

    fn encode_frame(
        &self,
        view: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        viewport: Viewport,
    ) -> wgpu::CommandBuffer {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
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
            let [x, y, width, height] = viewport.clip(None);
            pass.set_scissor_rect(x, y, width, height);
            pass.set_bind_group(0, &self.white.groups[0], &[]);
            pass.draw_indexed(0..6, 0, 0..1);
        }
        self.encode_meshes(&mut encoder, view, depth, viewport);
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("HUD"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
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
                let start = batch.start.max(1);
                if start == batch.end {
                    continue;
                }
                let texture = batch
                    .texture
                    .map_or(&self.white, |slot| self.textures[slot].as_ref().unwrap());
                let sampler = usize::from(batch.sampling == Sampling::Linear);
                pass.set_bind_group(0, &texture.groups[sampler], &[]);
                let [x, y, width, height] = batch.clip;
                pass.set_scissor_rect(x, y, width, height);
                pass.draw_indexed(start * 6..batch.end * 6, 0, 0..1);
            }
        }
        encoder.finish()
    }

    fn encode_meshes(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        viewport: Viewport,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("opaque scene"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        pass.set_viewport(
            viewport.x,
            viewport.y,
            viewport.width,
            viewport.height,
            0.0,
            1.0,
        );
        self.meshes.draw(&mut pass, viewport);
    }
}

fn make_indices(device: &wgpu::Device, quad_count: u32) -> Result<wgpu::Buffer, RendererError> {
    let size = usize::try_from(quad_count)
        .map_err(error)?
        .checked_mul(6 * 4)
        .ok_or_else(|| error("index limit overflow"))?;
    if u64::try_from(size).map_err(error)? > device.limits().max_buffer_size {
        return Err(error("index buffer exceeds device limits"));
    }
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(size).map_err(error)?;
    for quad in 0..quad_count {
        for offset in [0, 1, 2, 0, 2, 3] {
            bytes.extend_from_slice(&(quad * 4 + offset).to_ne_bytes());
        }
    }
    Ok(
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("indices"),
            contents: &bytes,
            usage: wgpu::BufferUsages::INDEX,
        }),
    )
}

fn surface_config(
    surface: &wgpu::Surface<'_>,
    adapter: &wgpu::Adapter,
    width: u32,
    height: u32,
) -> Result<wgpu::SurfaceConfiguration, RendererError> {
    let mut config = surface
        .get_default_config(adapter, width.max(1), height.max(1))
        .ok_or_else(|| error("surface unsupported"))?;
    if let Some(format) = surface
        .get_capabilities(adapter)
        .formats
        .into_iter()
        .find(wgpu::TextureFormat::is_srgb)
    {
        config.format = format;
    }
    let view_format = config.format.add_srgb_suffix();
    if !view_format.is_srgb() {
        return Err(error("sRGB surface view unavailable"));
    }
    config.view_formats = vec![view_format];
    config.present_mode = wgpu::PresentMode::Fifo;
    config.desired_maximum_frame_latency = u32::try_from(FRAMES_IN_FLIGHT).unwrap();
    Ok(config)
}

fn make_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    samplers: &[wgpu::Sampler; 2],
    image: TextureUpload<'_>,
    failure: &Failure,
) -> Result<Texture, RendererError> {
    let size = wgpu::Extent3d {
        width: image.width,
        height: image.height,
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
        image.rgba,
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
        width: image.width,
        height: image.height,
        bytes: image.rgba.len(),
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
