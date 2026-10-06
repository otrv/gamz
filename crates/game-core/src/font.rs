use crate::asset::AssetError;
use platform_api::render::{
    GlyphQuad, Image, PixelRect, Point, Radians, Scale, Size, TextureId, Transform,
};

const FIRST: u8 = b' ';
const GLYPHS: usize = 95;
const MAX_KERNING_PAIRS: usize = 1024;
pub const MAX_FONT_FILE_BYTES: usize = 10 + GLYPHS * 14 + MAX_KERNING_PAIRS * 4;
pub const MAX_TEXT_BYTES: usize = 4096;

#[derive(Clone, Copy)]
struct Glyph {
    source: PixelRect,
    offset_x: i16,
    offset_y: i16,
    advance: u16,
}

#[derive(Clone, Copy)]
struct Kerning {
    pair: u16,
    adjustment: i16,
}

pub struct Font {
    texture: TextureId,
    line_height: u16,
    glyphs: [Glyph; GLYPHS],
    kerning: [Kerning; MAX_KERNING_PAIRS],
    kerning_count: u16,
}

impl Font {
    pub fn from_file(
        bytes: &[u8],
        texture: TextureId,
        atlas: Image<'_>,
    ) -> Result<Self, AssetError> {
        if bytes.len() < 10 || &bytes[..4] != b"GFN1" {
            return Err(AssetError::InvalidFont);
        }
        let line_height = word(bytes, 4);
        let kerning_count = word(bytes, 8);
        if line_height == 0
            || line_height > 1024
            || word(bytes, 6) != 0
            || usize::from(kerning_count) > MAX_KERNING_PAIRS
            || bytes.len() != 10 + GLYPHS * 14 + usize::from(kerning_count) * 4
        {
            return Err(AssetError::InvalidFont);
        }
        let empty = Glyph {
            source: PixelRect::new(0, 0, 0, 0),
            offset_x: 0,
            offset_y: 0,
            advance: 0,
        };
        let mut font = Self {
            texture,
            line_height,
            glyphs: [empty; GLYPHS],
            kerning: [Kerning {
                pair: 0,
                adjustment: 0,
            }; MAX_KERNING_PAIRS],
            kerning_count,
        };
        for (index, glyph) in font.glyphs.iter_mut().enumerate() {
            let offset = 10 + index * 14;
            let x = word(bytes, offset);
            let y = word(bytes, offset + 2);
            let width = word(bytes, offset + 4);
            let height = word(bytes, offset + 6);
            let offset_x = signed_word(bytes, offset + 8);
            let offset_y = signed_word(bytes, offset + 10);
            let advance = word(bytes, offset + 12);
            if u32::from(x) + u32::from(width) > u32::from(atlas.width())
                || u32::from(y) + u32::from(height) > u32::from(atlas.height())
                || offset_x.unsigned_abs() > 1024
                || offset_y.unsigned_abs() > 1024
            {
                return Err(AssetError::InvalidFont);
            }
            *glyph = Glyph {
                source: PixelRect::new(x, y, width, height),
                offset_x,
                offset_y,
                advance,
            };
        }
        let mut previous_pair = None;
        for (index, kerning) in font.kerning[..usize::from(kerning_count)]
            .iter_mut()
            .enumerate()
        {
            let offset = 10 + GLYPHS * 14 + index * 4;
            let pair = word(bytes, offset);
            if usize::from(pair) >= GLYPHS * GLYPHS
                || previous_pair.is_some_and(|previous| pair <= previous)
            {
                return Err(AssetError::InvalidFont);
            }
            *kerning = Kerning {
                pair,
                adjustment: signed_word(bytes, offset + 2),
            };
            previous_pair = Some(pair);
        }
        Ok(font)
    }

    #[must_use]
    pub fn texture(&self) -> TextureId {
        self.texture
    }

    #[must_use]
    pub fn measure(&self, text: &str, scale: Scale) -> Size {
        self.layout(text, Point::new(0.0, 0.0), scale, |_, _| {})
    }

    pub fn prepare<'a>(
        &self,
        text: &str,
        origin: Point,
        scale: Scale,
        output: &'a mut [GlyphQuad],
    ) -> &'a [GlyphQuad] {
        let mut count = 0;
        let _ = self.layout(text, origin, scale, |source, transform| {
            assert!(count < output.len(), "glyph output exhausted");
            output[count] = GlyphQuad { source, transform };
            count += 1;
        });
        &output[..count]
    }

    fn layout(
        &self,
        text: &str,
        origin: Point,
        scale: Scale,
        mut emit: impl FnMut(PixelRect, Transform),
    ) -> Size {
        assert!(text.len() <= MAX_TEXT_BYTES);
        let scale = scale.get();
        let mut pen_x = 0.0_f32;
        let mut pen_y = 0.0_f32;
        let mut width = 0.0_f32;
        let mut previous = None;
        for character in text.chars() {
            if character == '\n' {
                width = width.max(pen_x);
                pen_x = 0.0;
                pen_y += f32::from(self.line_height) * scale;
                previous = None;
                continue;
            }
            let ascii = u8::try_from(character)
                .ok()
                .filter(|byte| (b' '..=b'~').contains(byte))
                .unwrap_or(b'?');
            let index = usize::from(ascii - FIRST);
            if let Some(previous) = previous {
                let pair = u16::try_from(previous * GLYPHS + index).unwrap();
                let kerning = &self.kerning[..usize::from(self.kerning_count)];
                if let Ok(found) = kerning.binary_search_by_key(&pair, |entry| entry.pair) {
                    pen_x += f32::from(kerning[found].adjustment) / 64.0 * scale;
                }
            }
            let glyph = self.glyphs[index];
            if glyph.source.width() != 0 && glyph.source.height() != 0 {
                let size = Size::new(
                    f32::from(glyph.source.width()) * scale,
                    f32::from(glyph.source.height()) * scale,
                );
                emit(
                    glyph.source,
                    Transform {
                        center: Point::new(
                            origin.x()
                                + pen_x
                                + f32::from(glyph.offset_x) * scale
                                + size.width() * 0.5,
                            origin.y()
                                + pen_y
                                + f32::from(glyph.offset_y) * scale
                                + size.height() * 0.5,
                        ),
                        size,
                        rotation: Radians::new(0.0),
                    },
                );
            }
            pen_x += f32::from(glyph.advance) / 64.0 * scale;
            width = width.max(pen_x);
            previous = Some(index);
        }
        Size::new(
            width,
            if text.is_empty() {
                0.0
            } else {
                pen_y + f32::from(self.line_height) * scale
            },
        )
    }
}

fn word(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn signed_word(bytes: &[u8], offset: usize) -> i16 {
    i16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}
