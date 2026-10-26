use super::FontError;
use platform_api::render::{Image, PixelRect};

const GLYPHS: usize = 95;
const MAX_KERNING_PAIRS: usize = 1024;
pub const MAX_FONT_FILE_BYTES: usize = 10 + GLYPHS * 14 + MAX_KERNING_PAIRS * 4;

#[derive(Clone, Copy)]
pub(super) struct Glyph {
    pub(super) source: PixelRect,
    pub(super) offset_x: i16,
    pub(super) offset_y: i16,
    pub(super) advance: u16,
}

#[derive(Clone, Copy)]
pub(super) struct Kerning {
    pub(super) pair: u16,
    pub(super) adjustment: i16,
}

pub(super) struct FontData {
    line_height: u16,
    glyphs: [Glyph; GLYPHS],
    kerning: [Kerning; MAX_KERNING_PAIRS],
    kerning_count: u16,
}

impl FontData {
    pub(super) fn line_height(&self) -> u16 {
        self.line_height
    }

    pub(super) fn glyphs(&self) -> &[Glyph; GLYPHS] {
        &self.glyphs
    }

    pub(super) fn kerning(&self) -> &[Kerning] {
        &self.kerning[..usize::from(self.kerning_count)]
    }
}

pub(super) fn read_font(bytes: &[u8], atlas: Image<'_>) -> Result<FontData, FontError> {
    let atlas_length = usize::from(atlas.width)
        .checked_mul(usize::from(atlas.height))
        .and_then(|pixels| pixels.checked_mul(4));
    if atlas.width == 0
        || atlas.height == 0
        || atlas_length != Some(atlas.rgba.len())
        || bytes.len() < 10
        || &bytes[..4] != b"GFN1"
    {
        return Err(FontError::InvalidData);
    }
    let line_height = word(bytes, 4);
    let kerning_count = word(bytes, 8);
    if line_height == 0
        || line_height > 1024
        || word(bytes, 6) != 0
        || usize::from(kerning_count) > MAX_KERNING_PAIRS
        || bytes.len() != 10 + GLYPHS * 14 + usize::from(kerning_count) * 4
    {
        return Err(FontError::InvalidData);
    }
    let empty = Glyph {
        source: PixelRect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        },
        offset_x: 0,
        offset_y: 0,
        advance: 0,
    };
    let mut data = FontData {
        line_height,
        glyphs: [empty; GLYPHS],
        kerning: [Kerning {
            pair: 0,
            adjustment: 0,
        }; MAX_KERNING_PAIRS],
        kerning_count,
    };
    let indexed_glyphs = data.glyphs.iter_mut().enumerate();
    for (index, glyph) in indexed_glyphs {
        let offset = 10 + index * 14;
        let x = word(bytes, offset);
        let y = word(bytes, offset + 2);
        let width = word(bytes, offset + 4);
        let height = word(bytes, offset + 6);
        let offset_x = signed_word(bytes, offset + 8);
        let offset_y = signed_word(bytes, offset + 10);
        let advance = word(bytes, offset + 12);
        if u32::from(x) + u32::from(width) > u32::from(atlas.width)
            || u32::from(y) + u32::from(height) > u32::from(atlas.height)
            || offset_x.unsigned_abs() > 1024
            || offset_y.unsigned_abs() > 1024
        {
            return Err(FontError::InvalidData);
        }
        *glyph = Glyph {
            source: PixelRect {
                x,
                y,
                width,
                height,
            },
            offset_x,
            offset_y,
            advance,
        };
    }
    let mut previous_pair = None;
    let indexed_kerning = data.kerning[..usize::from(kerning_count)]
        .iter_mut()
        .enumerate();
    for (index, kerning) in indexed_kerning {
        let offset = 10 + GLYPHS * 14 + index * 4;
        let pair = word(bytes, offset);
        if usize::from(pair) >= GLYPHS * GLYPHS
            || previous_pair.is_some_and(|previous| pair <= previous)
        {
            return Err(FontError::InvalidData);
        }
        *kerning = Kerning {
            pair,
            adjustment: signed_word(bytes, offset + 2),
        };
        previous_pair = Some(pair);
    }
    Ok(data)
}

fn word(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn signed_word(bytes: &[u8], offset: usize) -> i16 {
    i16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}
