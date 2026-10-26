use game_core::asset;
use game_core::font::{Font, FontError};
use game_core::render::{RenderCommands, RenderError};
use game_memory::Arena;
use math::{Radians, Rect, Vec2, Vec4};
use platform_api::render::{
    Image, PixelRect, RectStyle, RenderCommand, Sampling, TextureId, TexturedQuad,
};

const GLYPHS: usize = 95;
const ATLAS_WIDTH: usize = GLYPHS * 4;
static ATLAS: [u8; 8 + ATLAS_WIDTH * 5 * 4] = atlas_bytes();
static FONT: [u8; 10 + GLYPHS * 14] = font_bytes();

pub(super) fn image() -> Image<'static> {
    asset::read_rgba(&ATLAS).expect("embedded HUD atlas is valid RGBA")
}

pub(super) fn font() -> Result<Font, FontError> {
    Font::try_new(&FONT, image())
}

pub(super) fn draw<'a>(
    font: &Font,
    label: &str,
    color: Vec4,
    arena: &mut Arena<'a>,
    commands: &mut RenderCommands<'a>,
) -> Result<(), RenderError> {
    let scale = 3.0;
    let metrics = font
        .measure(label, scale)
        .expect("HUD labels fit the text budget");
    let origin = Vec2::new((1280.0 - metrics.advance_size.x) * 0.5, 24.0);
    let glyphs = arena.push_slice(
        metrics.glyph_count,
        TexturedQuad {
            source: PixelRect {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            },
            center: origin,
            size: Vec2::ZERO,
            rotation: Radians::new(0.0),
        },
    );
    let prepared = font
        .prepare(label, origin, scale, glyphs)
        .expect("measured HUD glyph storage and canvas placement fit");
    commands.push(RenderCommand::DrawQuads {
        quads: prepared.glyphs,
        texture: TextureId(0),
        tint: color,
        sampling: Sampling::Nearest,
    })?;
    commands.push(RenderCommand::DrawRect {
        rect: Rect {
            origin: Vec2::new(origin.x, origin.y + prepared.metrics.advance_size.y),
            size: Vec2::new(prepared.metrics.advance_size.x, 2.0),
        },
        style: RectStyle::Filled(color),
    })
}

const fn font_bytes() -> [u8; 10 + GLYPHS * 14] {
    let mut bytes = [0; 10 + GLYPHS * 14];
    bytes[0] = b'G';
    bytes[1] = b'F';
    bytes[2] = b'N';
    bytes[3] = b'1';
    bytes[4] = 7;
    let mut index = 0;
    while index < GLYPHS {
        let offset = 10 + index * 14;
        let x = (index * 4).to_le_bytes();
        bytes[offset] = x[0];
        bytes[offset + 1] = x[1];
        if index != 0 {
            bytes[offset + 4] = 3;
            bytes[offset + 6] = 5;
        }
        bytes[offset + 13] = 1;
        index += 1;
    }
    bytes
}

const fn atlas_bytes() -> [u8; 8 + ATLAS_WIDTH * 5 * 4] {
    let mut bytes = [0; 8 + ATLAS_WIDTH * 5 * 4];
    bytes[0] = b'R';
    bytes[1] = b'G';
    bytes[2] = b'B';
    bytes[3] = b'A';
    let width = ATLAS_WIDTH.to_le_bytes();
    bytes[4] = width[0];
    bytes[5] = width[1];
    bytes[6] = 5;
    let letters = b"AEGILMNOPRSTUVWY?";
    let rows = [
        [2, 5, 7, 5, 5],
        [7, 4, 6, 4, 7],
        [7, 4, 5, 5, 7],
        [7, 2, 2, 2, 7],
        [4, 4, 4, 4, 7],
        [5, 7, 7, 5, 5],
        [5, 7, 7, 7, 5],
        [7, 5, 5, 5, 7],
        [7, 5, 7, 4, 4],
        [6, 5, 6, 5, 5],
        [7, 4, 7, 1, 7],
        [7, 2, 2, 2, 2],
        [5, 5, 5, 5, 7],
        [5, 5, 5, 5, 2],
        [5, 5, 7, 7, 5],
        [5, 5, 2, 2, 2],
        [7, 1, 3, 0, 2],
    ];
    let mut letter = 0;
    while letter < letters.len() {
        let index = (letters[letter] - b' ') as usize;
        let mut y = 0;
        while y < 5 {
            let mut x = 0;
            while x < 3 {
                if rows[letter][y] & (4 >> x) != 0 {
                    let pixel = 8 + (y * ATLAS_WIDTH + index * 4 + x) * 4;
                    bytes[pixel] = 255;
                    bytes[pixel + 1] = 255;
                    bytes[pixel + 2] = 255;
                    bytes[pixel + 3] = 255;
                }
                x += 1;
            }
            y += 1;
        }
        letter += 1;
    }
    bytes
}
