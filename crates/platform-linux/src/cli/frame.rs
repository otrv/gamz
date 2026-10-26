use math::{Rect, Vec2, Vec4};
use platform_api::render::{Frame, RectStyle, RenderCommand};

use crate::limits::{MAX_COMMANDS, MAX_QUADS};

pub(super) fn validate(frame: &Frame<'_>) -> Result<(), &'static str> {
    if !finite(frame.canvas_size)
        || frame.canvas_size.x <= 0.0
        || frame.canvas_size.y <= 0.0
        || !color(frame.clear_color)
        || frame.commands.len() > MAX_COMMANDS
    {
        return Err("invalid frame or command capacity exceeded");
    }
    let mut quads = 0_usize;
    for command in frame.commands {
        let count = match *command {
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
