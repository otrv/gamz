use math::{Radians, Rect, Vec2, Vec4};

pub struct Frame<'a> {
    pub canvas_size: Vec2,
    pub clear_color: Vec4,
    pub commands: &'a [RenderCommand<'a>],
}

#[derive(Clone, Copy)]
pub enum RenderCommand<'a> {
    SetCamera {
        origin: Vec2,
        zoom: f32,
    },
    SetClip(Option<Rect>),
    DrawRect {
        rect: Rect,
        style: RectStyle,
    },
    DrawLine {
        start: Vec2,
        end: Vec2,
        width: f32,
        color: Vec4,
    },
    DrawQuads {
        texture: TextureId,
        quads: &'a [TexturedQuad],
        tint: Vec4,
        sampling: Sampling,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct TexturedQuad {
    pub source: PixelRect,
    pub center: Vec2,
    pub size: Vec2,
    pub rotation: Radians,
}

#[derive(Clone, Copy, Debug)]
pub enum RectStyle {
    Filled(Vec4),
    Outline { color: Vec4, width: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sampling {
    Nearest,
    Linear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureId(pub u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelRect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Clone, Copy)]
pub struct Image<'a> {
    pub width: u16,
    pub height: u16,
    pub rgba: &'a [u8],
}

#[derive(Clone, Copy)]
pub struct TextureUpload<'a> {
    pub id: TextureId,
    pub width: u32,
    pub height: u32,
    pub rgba: &'a [u8],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureError {
    InvalidImage,
    InvalidId,
    CapacityExceeded,
    Unavailable,
}
