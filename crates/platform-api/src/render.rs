pub const MAX_COMMANDS: usize = 16_384;
pub const MAX_QUADS: usize = 65_536;
pub const MAX_TEXTURES: usize = 64;
pub const MAX_TEXTURE_SIDE: u16 = 2048;
pub const MAX_TEXTURE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    x: f32,
    y: f32,
}

impl Point {
    #[must_use]
    pub fn new(x: f32, y: f32) -> Self {
        assert!(x.is_finite() && y.is_finite() && x.abs() <= 1.0e12 && y.abs() <= 1.0e12);
        Self { x, y }
    }

    #[must_use]
    pub fn x(self) -> f32 {
        self.x
    }

    #[must_use]
    pub fn y(self) -> f32 {
        self.y
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Size {
    width: f32,
    height: f32,
}

impl Size {
    #[must_use]
    pub fn new(width: f32, height: f32) -> Self {
        assert!((0.0..=1.0e9).contains(&width) && (0.0..=1.0e9).contains(&height));
        Self { width, height }
    }

    #[must_use]
    pub fn width(self) -> f32 {
        self.width
    }

    #[must_use]
    pub fn height(self) -> f32 {
        self.height
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    origin: Point,
    size: Size,
}

impl Rect {
    #[must_use]
    pub fn new(origin: Point, size: Size) -> Self {
        Self { origin, size }
    }

    #[must_use]
    pub fn origin(self) -> Point {
        self.origin
    }

    #[must_use]
    pub fn size(self) -> Size {
        self.size
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CanvasRect(Rect);

impl CanvasRect {
    #[must_use]
    pub fn new(bounds: Rect) -> Self {
        Self(bounds)
    }

    #[must_use]
    pub fn bounds(self) -> Rect {
        self.0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Canvas {
    width: u16,
    height: u16,
}

impl Canvas {
    #[must_use]
    pub fn new(width: u16, height: u16) -> Self {
        assert!((1..=16_384).contains(&width) && (1..=16_384).contains(&height));
        Self { width, height }
    }

    #[must_use]
    pub fn width(self) -> f32 {
        f32::from(self.width)
    }

    #[must_use]
    pub fn height(self) -> f32 {
        f32::from(self.height)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color(pub u8, pub u8, pub u8, pub u8);

impl Color {
    pub const WHITE: Self = Self(255, 255, 255, 255);
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scale(f32);

impl Scale {
    #[must_use]
    pub fn new(value: f32) -> Self {
        assert!((0.01..=32.0).contains(&value));
        Self(value)
    }

    #[must_use]
    pub fn get(self) -> f32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Radians(f32);

impl Radians {
    #[must_use]
    pub fn new(value: f32) -> Self {
        assert!(value.is_finite() && value.abs() <= core::f32::consts::TAU);
        Self(value)
    }

    #[must_use]
    pub fn get(self) -> f32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeWidth(f32);

impl StrokeWidth {
    #[must_use]
    pub fn new(value: f32) -> Self {
        assert!((0.01..=1024.0).contains(&value));
        Self(value)
    }

    #[must_use]
    pub fn get(self) -> f32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub center: Point,
    pub size: Size,
    pub rotation: Radians,
}

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub origin: Point,
    pub zoom: Scale,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            origin: Point::new(0.0, 0.0),
            zoom: Scale::new(1.0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureId(u16);

impl TextureId {
    #[must_use]
    pub fn new(slot: u16) -> Self {
        assert!(usize::from(slot) < MAX_TEXTURES);
        Self(slot)
    }

    #[must_use]
    pub fn index(self) -> usize {
        usize::from(self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelRect {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

impl PixelRect {
    #[must_use]
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        assert!(u32::from(x) + u32::from(width) <= u32::from(MAX_TEXTURE_SIDE));
        assert!(u32::from(y) + u32::from(height) <= u32::from(MAX_TEXTURE_SIDE));
        Self {
            x,
            y,
            width,
            height,
        }
    }

    #[must_use]
    pub fn x(self) -> u16 {
        self.x
    }
    #[must_use]
    pub fn y(self) -> u16 {
        self.y
    }
    #[must_use]
    pub fn width(self) -> u16 {
        self.width
    }
    #[must_use]
    pub fn height(self) -> u16 {
        self.height
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sampling {
    Nearest,
    Linear,
}

#[derive(Clone, Copy, Debug)]
pub struct Sprite {
    pub texture: TextureId,
    pub source: PixelRect,
    pub transform: Transform,
    pub tint: Color,
    pub sampling: Sampling,
}

#[derive(Clone, Copy, Debug)]
pub enum RectStyle {
    Filled(Color),
    Outline { color: Color, width: StrokeWidth },
}

#[derive(Clone, Copy)]
pub struct GlyphQuad {
    pub source: PixelRect,
    pub transform: Transform,
}

#[derive(Clone, Copy)]
pub struct Text<'a> {
    glyphs: &'a [GlyphQuad],
    pub texture: TextureId,
    pub color: Color,
    pub sampling: Sampling,
}

impl<'a> Text<'a> {
    #[must_use]
    pub fn new(
        glyphs: &'a [GlyphQuad],
        texture: TextureId,
        color: Color,
        sampling: Sampling,
    ) -> Self {
        assert!(glyphs.len() <= MAX_QUADS);
        Self {
            glyphs,
            texture,
            color,
            sampling,
        }
    }

    #[must_use]
    pub fn glyphs(self) -> &'a [GlyphQuad] {
        self.glyphs
    }
}

#[derive(Clone, Copy)]
pub enum RenderCommand<'a> {
    SetCamera(Camera),
    SetClip(Option<CanvasRect>),
    DrawSprite(Sprite),
    DrawRect {
        rect: Rect,
        style: RectStyle,
    },
    DrawLine {
        start: Point,
        end: Point,
        width: StrokeWidth,
        color: Color,
    },
    DrawText(Text<'a>),
}

pub struct Frame<'a> {
    pub canvas: Canvas,
    pub clear: Color,
    pub commands: &'a [RenderCommand<'a>],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidImage;

#[derive(Clone, Copy)]
pub struct Image<'a> {
    width: u16,
    height: u16,
    rgba: &'a [u8],
}

impl<'a> Image<'a> {
    pub fn new(width: u16, height: u16, rgba: &'a [u8]) -> Result<Self, InvalidImage> {
        if width == 0
            || height == 0
            || width > MAX_TEXTURE_SIDE
            || height > MAX_TEXTURE_SIDE
            || rgba.len() != usize::from(width) * usize::from(height) * 4
        {
            return Err(InvalidImage);
        }
        Ok(Self {
            width,
            height,
            rgba,
        })
    }

    #[must_use]
    pub fn width(self) -> u16 {
        self.width
    }
    #[must_use]
    pub fn height(self) -> u16 {
        self.height
    }
    #[must_use]
    pub fn rgba(self) -> &'a [u8] {
        self.rgba
    }
}

#[derive(Clone, Copy)]
pub struct TextureUpload<'a> {
    pub id: TextureId,
    pub image: Image<'a>,
}
