use math::Vec4;
use platform_api::render::{Frame, RenderCommand};

use super::geometry::Viewport;
use super::{Limits, RendererError, error};

const VERTEX_BYTES: usize = 28;
const MATRIX_BYTES: usize = 64;

struct Draw {
    start: u32,
    end: u32,
    base_vertex: i32,
}

pub(super) struct Meshes {
    pipeline: wgpu::RenderPipeline,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    transforms: wgpu::Buffer,
    vertex_bytes: Vec<u8>,
    index_bytes: Vec<u8>,
    transform_bytes: Vec<u8>,
    draws: Vec<Draw>,
    limits: Limits,
}

impl Meshes {
    pub(super) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        limits: Limits,
    ) -> Result<Self, RendererError> {
        if limits.mesh_vertices > i32::MAX as usize
            || limits.mesh_indices > u32::MAX as usize
            || limits.commands > u32::MAX as usize
        {
            return Err(error("mesh limits exceed draw ranges"));
        }
        let (vertices, vertex_bytes) = buffer(
            device,
            limits.mesh_vertices,
            VERTEX_BYTES,
            wgpu::BufferUsages::VERTEX,
        )?;
        let (indices, index_bytes) =
            buffer(device, limits.mesh_indices, 4, wgpu::BufferUsages::INDEX)?;
        let (transforms, transform_bytes) = buffer(
            device,
            limits.commands,
            MATRIX_BYTES,
            wgpu::BufferUsages::VERTEX,
        )?;
        let mut draws = Vec::new();
        draws.try_reserve_exact(limits.commands).map_err(error)?;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mesh"),
            source: wgpu::ShaderSource::Wgsl(include_str!("mesh.wgsl").into()),
        });
        let vertex_attributes = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];
        let matrix_attributes = wgpu::vertex_attr_array![
            2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4
        ];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("opaque meshes"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                buffers: &[
                    Some(wgpu::VertexBufferLayout {
                        array_stride: VERTEX_BYTES as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &vertex_attributes,
                    }),
                    Some(wgpu::VertexBufferLayout {
                        array_stride: MATRIX_BYTES as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &matrix_attributes,
                    }),
                ],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        Ok(Self {
            pipeline,
            vertices,
            indices,
            transforms,
            vertex_bytes,
            index_bytes,
            transform_bytes,
            draws,
            limits,
        })
    }

    pub(super) fn prepare(&mut self, frame: &Frame<'_>) -> Result<(), RendererError> {
        self.vertex_bytes.clear();
        self.index_bytes.clear();
        self.transform_bytes.clear();
        self.draws.clear();
        let mut camera = None;
        for command in frame.commands {
            match *command {
                RenderCommand::SetCamera3D { view_projection } => {
                    if !view_projection
                        .to_cols_array()
                        .into_iter()
                        .all(f32::is_finite)
                    {
                        return Err(error("invalid 3D camera"));
                    }
                    camera = Some(view_projection);
                }
                RenderCommand::DrawMesh {
                    vertices,
                    indices,
                    model,
                } => {
                    let camera = camera.ok_or_else(|| error("mesh requires a 3D camera"))?;
                    let base_vertex = self.vertex_bytes.len() / VERTEX_BYTES;
                    let start = self.index_bytes.len() / 4;
                    if vertices.len() > self.limits.mesh_vertices - base_vertex
                        || indices.len() > self.limits.mesh_indices - start
                        || self.draws.len() == self.limits.commands
                    {
                        return Err(error("mesh capacity exceeded"));
                    }
                    if !indices.len().is_multiple_of(3)
                        || !model.to_cols_array().into_iter().all(f32::is_finite)
                    {
                        return Err(error("invalid mesh"));
                    }
                    let transform = camera * model;
                    let columns = transform.to_cols_array();
                    if !columns.into_iter().all(f32::is_finite) {
                        return Err(error("mesh transform exceeds GPU range"));
                    }
                    for vertex in vertices {
                        let p = vertex.position;
                        let c = vertex.color;
                        let clip = transform * Vec4::new(p.x, p.y, p.z, 1.0);
                        if ![p.x, p.y, p.z, clip.x, clip.y, clip.z, clip.w]
                            .into_iter()
                            .all(f32::is_finite)
                            || ![c.x, c.y, c.z, c.w]
                                .into_iter()
                                .all(|v| (0.0..=1.0).contains(&v))
                        {
                            return Err(error("invalid opaque mesh vertex"));
                        }
                        for value in [p.x, p.y, p.z, c.x, c.y, c.z, c.w] {
                            self.vertex_bytes.extend_from_slice(&value.to_ne_bytes());
                        }
                    }
                    for &index in indices {
                        if usize::try_from(index).map_err(error)? >= vertices.len() {
                            return Err(error("mesh index out of bounds"));
                        }
                        self.index_bytes.extend_from_slice(&index.to_ne_bytes());
                    }
                    for value in columns {
                        self.transform_bytes.extend_from_slice(&value.to_ne_bytes());
                    }
                    self.draws.push(Draw {
                        start: u32::try_from(start).unwrap(),
                        end: u32::try_from(start + indices.len()).unwrap(),
                        base_vertex: i32::try_from(base_vertex).unwrap(),
                    });
                }
                RenderCommand::SetCamera { .. }
                | RenderCommand::SetClip(_)
                | RenderCommand::DrawRect { .. }
                | RenderCommand::DrawLine { .. }
                | RenderCommand::DrawQuads { .. } => {}
            }
        }
        Ok(())
    }

    pub(super) fn upload(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.vertices, 0, &self.vertex_bytes);
        queue.write_buffer(&self.indices, 0, &self.index_bytes);
        queue.write_buffer(&self.transforms, 0, &self.transform_bytes);
    }

    pub(super) fn vertex_bytes(&self) -> usize {
        self.vertex_bytes.len()
    }

    pub(super) fn draw_count(&self) -> usize {
        self.draws.len()
    }

    pub(super) fn draw(&self, pass: &mut wgpu::RenderPass<'_>, viewport: Viewport) {
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_vertex_buffer(1, self.transforms.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        let [x, y, width, height] = viewport.clip(None);
        pass.set_scissor_rect(x, y, width, height);
        for (instance, draw) in self.draws.iter().enumerate() {
            let instance = u32::try_from(instance).unwrap();
            pass.draw_indexed(
                draw.start..draw.end,
                draw.base_vertex,
                instance..instance + 1,
            );
        }
    }
}

fn buffer(
    device: &wgpu::Device,
    count: usize,
    stride: usize,
    usage: wgpu::BufferUsages,
) -> Result<(wgpu::Buffer, Vec<u8>), RendererError> {
    let bytes = count
        .checked_mul(stride)
        .ok_or_else(|| error("mesh buffer size overflow"))?;
    let size = u64::try_from(bytes).map_err(error)?.max(4);
    if size > device.limits().max_buffer_size {
        return Err(error("mesh buffer exceeds device limit"));
    }
    let mut staging = Vec::new();
    staging.try_reserve_exact(bytes).map_err(error)?;
    Ok((
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh buffer"),
            size,
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }),
        staging,
    ))
}
