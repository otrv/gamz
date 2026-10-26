mod data;

use data::FontData;
use math::{Radians, Vec2};
use platform_api::render::{Image, TexturedQuad};

pub use data::MAX_FONT_FILE_BYTES;

#[cfg(test)]
mod tests;

const FIRST: u8 = b' ';
pub const MAX_TEXT_BYTES: usize = 4096;

pub struct Font {
    data: FontData,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontError {
    InvalidData,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutError {
    TextTooLong,
    OutputTooSmall,
    OutOfBounds,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextMetrics {
    pub advance_size: Vec2,
    pub glyph_count: usize,
}

pub struct PreparedText<'a> {
    pub glyphs: &'a [TexturedQuad],
    pub metrics: TextMetrics,
}

impl Font {
    pub fn try_new(bytes: &[u8], atlas: Image<'_>) -> Result<Self, FontError> {
        Ok(Self {
            data: data::read_font(bytes, atlas)?,
        })
    }

    pub fn measure(&self, text: &str, scale: f32) -> Result<TextMetrics, LayoutError> {
        self.layout(text, Vec2::ZERO, scale, |_| Ok(()))
    }

    pub fn prepare<'a>(
        &self,
        text: &str,
        origin: Vec2,
        scale: f32,
        output: &'a mut [TexturedQuad],
    ) -> Result<PreparedText<'a>, LayoutError> {
        let mut count = 0;
        let metrics = self.layout(text, origin, scale, |quad| {
            let slot = output.get_mut(count).ok_or(LayoutError::OutputTooSmall)?;
            *slot = quad;
            count += 1;
            Ok(())
        })?;
        Ok(PreparedText {
            glyphs: &output[..count],
            metrics,
        })
    }

    fn layout(
        &self,
        text: &str,
        origin: Vec2,
        scale: f32,
        mut emit: impl FnMut(TexturedQuad) -> Result<(), LayoutError>,
    ) -> Result<TextMetrics, LayoutError> {
        if text.len() > MAX_TEXT_BYTES {
            return Err(LayoutError::TextTooLong);
        }
        if !origin.x.is_finite() || !origin.y.is_finite() || !scale.is_finite() || scale <= 0.0 {
            return Err(LayoutError::OutOfBounds);
        }
        let mut pen_x = 0.0_f32;
        let mut pen_y = 0.0_f32;
        let mut width = 0.0_f32;
        let mut glyph_count = 0;
        let mut previous = None;
        for character in text.chars() {
            if character == '\n' {
                width = width.max(pen_x);
                pen_x = 0.0;
                pen_y += f32::from(self.data.line_height()) * scale;
                if !pen_y.is_finite() {
                    return Err(LayoutError::OutOfBounds);
                }
                previous = None;
                continue;
            }
            let ascii = u8::try_from(character)
                .ok()
                .filter(|byte| (b' '..=b'~').contains(byte))
                .unwrap_or(b'?');
            let index = usize::from(ascii - FIRST);
            if let Some(previous) = previous {
                let pair = u16::try_from(previous * self.data.glyphs().len() + index).unwrap();
                let kerning = self.data.kerning();
                if let Ok(found) = kerning.binary_search_by_key(&pair, |entry| entry.pair) {
                    pen_x += f32::from(kerning[found].adjustment) / 64.0 * scale;
                }
            }
            let glyph = self.data.glyphs()[index];
            if glyph.source.width != 0 && glyph.source.height != 0 {
                let size = Vec2::new(
                    f32::from(glyph.source.width) * scale,
                    f32::from(glyph.source.height) * scale,
                );
                let center = Vec2::new(
                    origin.x + pen_x + f32::from(glyph.offset_x) * scale + size.x * 0.5,
                    origin.y + pen_y + f32::from(glyph.offset_y) * scale + size.y * 0.5,
                );
                if !size.x.is_finite()
                    || !size.y.is_finite()
                    || !center.x.is_finite()
                    || !center.y.is_finite()
                {
                    return Err(LayoutError::OutOfBounds);
                }
                emit(TexturedQuad {
                    source: glyph.source,
                    center,
                    size,
                    rotation: Radians::new(0.0),
                })?;
                glyph_count += 1;
            }
            pen_x += f32::from(glyph.advance) / 64.0 * scale;
            if !pen_x.is_finite() {
                return Err(LayoutError::OutOfBounds);
            }
            width = width.max(pen_x);
            previous = Some(index);
        }
        let height = if text.is_empty() {
            0.0
        } else {
            pen_y + f32::from(self.data.line_height()) * scale
        };
        if !height.is_finite() {
            return Err(LayoutError::OutOfBounds);
        }
        Ok(TextMetrics {
            advance_size: Vec2::new(width, height),
            glyph_count,
        })
    }
}
