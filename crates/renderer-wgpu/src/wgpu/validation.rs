use super::{Limits, RendererError, Texture, error};
use math::{Rect, Vec2, Vec4};
use platform_api::render::{Frame, RectStyle, RenderCommand, TextureError, TextureUpload};

pub(super) fn image(upload: TextureUpload<'_>) -> Result<(), TextureError> {
    let bytes = usize::try_from(upload.width)
        .ok()
        .and_then(|width| {
            usize::try_from(upload.height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4));
    if upload.width == 0
        || upload.height == 0
        || upload.width.checked_mul(4).is_none()
        || bytes != Some(upload.rgba.len())
    {
        return Err(TextureError::InvalidImage);
    }
    Ok(())
}

pub(super) fn frame(
    frame: &Frame<'_>,
    limits: Limits,
    textures: &[Option<Texture>],
) -> Result<(), RendererError> {
    if !positive(frame.canvas_size.x) || !positive(frame.canvas_size.y) || !color(frame.clear_color)
    {
        return Err(error("invalid canvas or clear color"));
    }
    if frame.commands.len() > limits.commands {
        return Err(error("command budget exhausted"));
    }
    let mut quads = 0_usize;
    for command in frame.commands {
        let (valid, count) = match *command {
            RenderCommand::SetCamera { origin, zoom } => (point(origin) && positive(zoom), 0),
            RenderCommand::SetClip(clip) => (clip.is_none_or(rect), 0),
            RenderCommand::DrawRect {
                rect: bounds,
                style,
            } => match style {
                RectStyle::Filled(tint) => (rect(bounds) && color(tint), 1),
                RectStyle::Outline { color: tint, width } => {
                    (rect(bounds) && color(tint) && positive(width), 4)
                }
            },
            RenderCommand::DrawLine {
                start,
                end,
                width,
                color: tint,
            } => (
                point(start) && point(end) && positive(width) && color(tint),
                1,
            ),
            RenderCommand::DrawQuads {
                texture,
                quads: items,
                tint,
                ..
            } => {
                let texture = textures
                    .get(usize::from(texture.0))
                    .and_then(Option::as_ref)
                    .ok_or_else(|| error("texture is not resident"))?;
                if items.len() > limits.quads.saturating_sub(quads) {
                    return Err(error("quad budget exhausted"));
                }
                for item in items {
                    if !point(item.center)
                        || !size(item.size)
                        || !item.rotation.get().is_finite()
                        || u32::from(item.source.x) + u32::from(item.source.width) > texture.width
                        || u32::from(item.source.y) + u32::from(item.source.height) > texture.height
                    {
                        return Err(error("invalid textured quad"));
                    }
                }
                (color(tint), items.len())
            }
        };
        if !valid {
            return Err(error("invalid render command"));
        }
        quads = quads
            .checked_add(count)
            .filter(|&count| count <= limits.quads)
            .ok_or_else(|| error("quad budget exhausted"))?;
    }
    Ok(())
}

fn positive(value: f32) -> bool {
    value.is_finite() && value > 0.0
}

fn point(value: Vec2) -> bool {
    value.x.is_finite() && value.y.is_finite()
}

fn size(value: Vec2) -> bool {
    point(value) && value.x >= 0.0 && value.y >= 0.0
}

fn rect(value: Rect) -> bool {
    point(value.origin) && size(value.size)
}

fn color(value: Vec4) -> bool {
    [value.x, value.y, value.z, value.w]
        .into_iter()
        .all(|channel| (0.0..=1.0).contains(&channel))
}
