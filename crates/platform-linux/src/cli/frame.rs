use math::{Rect, Vec2, Vec4};
use platform_api::render::{Frame, RectStyle, RenderCommand};

use crate::limits::{MAX_COMMANDS, MAX_MESH_INDICES, MAX_MESH_VERTICES, MAX_QUADS};

pub(super) fn validate(frame: &Frame<'_>) -> Result<(), &'static str> {
    if !finite(frame.canvas_size)
        || frame.canvas_size.x <= 0.0
        || frame.canvas_size.y <= 0.0
        || !color(frame.clear_color)
        || frame.commands.len() > MAX_COMMANDS
    {
        return Err("invalid frame or command capacity exceeded");
    }
    validate_meshes(frame)?;
    let mut quads = 0_usize;
    for command in frame.commands {
        let count = match *command {
            RenderCommand::SetCamera3D { .. } | RenderCommand::DrawMesh { .. } => 0,
            RenderCommand::SetCamera { origin, zoom } => {
                if !finite(origin) || !zoom.is_finite() || zoom <= 0.0 {
                    return Err("invalid camera");
                }
                0
            }
            RenderCommand::SetClip(clip) => {
                if clip.is_some_and(|value| !rect(value)) {
                    return Err("invalid clip");
                }
                0
            }
            RenderCommand::DrawRect {
                rect: bounds,
                style,
            } => {
                if !rect(bounds) {
                    return Err("invalid rectangle");
                }
                match style {
                    RectStyle::Filled(tint) => {
                        if !color(tint) {
                            return Err("invalid color");
                        }
                        1
                    }
                    RectStyle::Outline { color: tint, width } => {
                        if !color(tint) || !width.is_finite() || width <= 0.0 {
                            return Err("invalid outline");
                        }
                        4
                    }
                }
            }
            RenderCommand::DrawLine {
                start,
                end,
                width,
                color: tint,
            } => {
                if !finite(start)
                    || !finite(end)
                    || !width.is_finite()
                    || width <= 0.0
                    || !color(tint)
                {
                    return Err("invalid line");
                }
                1
            }
            RenderCommand::DrawQuads {
                quads: batch, tint, ..
            } => {
                if !color(tint) || batch.len() > MAX_QUADS {
                    return Err("invalid quad batch");
                }
                for quad in batch {
                    if !finite(quad.center)
                        || !finite(quad.size)
                        || quad.size.x < 0.0
                        || quad.size.y < 0.0
                        || !quad.rotation.get().is_finite()
                    {
                        return Err("invalid quad");
                    }
                }
                batch.len()
            }
        };
        quads = quads
            .checked_add(count)
            .filter(|&total| total <= MAX_QUADS)
            .ok_or("quad capacity exceeded")?;
    }
    Ok(())
}

fn validate_meshes(frame: &Frame<'_>) -> Result<(), &'static str> {
    let mut mesh_vertices = 0;
    let mut mesh_indices = 0;
    let mut camera = None;
    for command in frame.commands {
        match *command {
            RenderCommand::SetCamera3D { view_projection } => {
                if !view_projection
                    .to_cols_array()
                    .into_iter()
                    .all(f32::is_finite)
                {
                    return Err("invalid 3D camera");
                }
                camera = Some(view_projection);
            }
            RenderCommand::DrawMesh {
                vertices,
                indices,
                model,
            } => {
                let camera = camera.ok_or("mesh requires a 3D camera")?;
                if vertices.len() > MAX_MESH_VERTICES - mesh_vertices
                    || indices.len() > MAX_MESH_INDICES - mesh_indices
                {
                    return Err("mesh capacity exceeded");
                }
                mesh_vertices += vertices.len();
                mesh_indices += indices.len();
                let transform = camera * model;
                if !indices.len().is_multiple_of(3)
                    || !model.to_cols_array().into_iter().all(f32::is_finite)
                    || !transform.to_cols_array().into_iter().all(f32::is_finite)
                {
                    return Err("invalid mesh transform or triangle indices");
                }
                for vertex in vertices {
                    let p = vertex.position;
                    let clip = transform * Vec4::new(p.x, p.y, p.z, 1.0);
                    if ![p.x, p.y, p.z, clip.x, clip.y, clip.z, clip.w]
                        .into_iter()
                        .all(f32::is_finite)
                        || !color(vertex.color)
                    {
                        return Err("invalid opaque mesh vertex");
                    }
                }
                for &index in indices {
                    if usize::try_from(index).map_err(|_| "invalid mesh index")? >= vertices.len() {
                        return Err("mesh index out of bounds");
                    }
                }
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

fn finite(value: Vec2) -> bool {
    value.x.is_finite() && value.y.is_finite()
}
fn color(value: Vec4) -> bool {
    [value.x, value.y, value.z, value.w]
        .iter()
        .all(|channel| (0.0..=1.0).contains(channel))
}
fn rect(value: Rect) -> bool {
    finite(value.origin) && finite(value.size) && value.size.x >= 0.0 && value.size.y >= 0.0
}
