mod data;

use data::FontData;
use math::Radians;
use platform_api::render::{GlyphQuad, Image, PixelRect, Point, Scale, Size, Transform};

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
    pub advance_size: Size,
    pub glyph_count: usize,
}

pub struct PreparedText<'a> {
    pub glyphs: &'a [GlyphQuad],
    pub metrics: TextMetrics,
}

impl Font {
    pub fn try_new(bytes: &[u8], atlas: Image<'_>) -> Result<Self, FontError> {
        Ok(Self {
            data: data::read_font(bytes, atlas)?,
        })
    }

    pub fn measure(&self, text: &str, scale: Scale) -> Result<TextMetrics, LayoutError> {
        self.layout(text, Point::new(0.0, 0.0), scale, |_, _| Ok(()))
    }

    pub fn prepare<'a>(
        &self,
        text: &str,
        origin: Point,
        scale: Scale,
        output: &'a mut [GlyphQuad],
    ) -> Result<PreparedText<'a>, LayoutError> {
        let mut count = 0;
        let metrics = self.layout(text, origin, scale, |source, transform| {
            let slot = output.get_mut(count).ok_or(LayoutError::OutputTooSmall)?;
            *slot = GlyphQuad { source, transform };
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
        origin: Point,
        scale: Scale,
        mut emit: impl FnMut(PixelRect, Transform) -> Result<(), LayoutError>,
    ) -> Result<TextMetrics, LayoutError> {
        if text.len() > MAX_TEXT_BYTES {
            return Err(LayoutError::TextTooLong);
        }
        let scale = scale.get();
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
            if glyph.source.width() != 0 && glyph.source.height() != 0 {
                let size = Size::new(
                    f32::from(glyph.source.width()) * scale,
                    f32::from(glyph.source.height()) * scale,
                );
                emit(
                    glyph.source,
                    Transform {
                        center: Point::try_new(
                            origin.x()
                                + pen_x
                                + f32::from(glyph.offset_x) * scale
                                + size.width() * 0.5,
                            origin.y()
                                + pen_y
                                + f32::from(glyph.offset_y) * scale
                                + size.height() * 0.5,
                        )
                        .map_err(|_| LayoutError::OutOfBounds)?,
                        size,
                        rotation: Radians::new(0.0),
                    },
                )?;
                glyph_count += 1;
            }
            pen_x += f32::from(glyph.advance) / 64.0 * scale;
            width = width.max(pen_x);
            previous = Some(index);
        }
        Ok(TextMetrics {
            advance_size: Size::new(
                width,
                if text.is_empty() {
                    0.0
                } else {
                    pen_y + f32::from(self.data.line_height()) * scale
                },
            ),
            glyph_count,
        })
    }
}
