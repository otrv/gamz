use crate::asset::font::FontData;
use platform_api::render::{
    GlyphQuad, PixelRect, Point, Radians, Scale, Size, TextureId, Transform,
};

const FIRST: u8 = b' ';
pub const MAX_TEXT_BYTES: usize = 4096;

pub struct Font {
    texture: TextureId,
    data: FontData,
}

impl Font {
    pub(crate) fn new(texture: TextureId, data: FontData) -> Self {
        Self { texture, data }
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
                pen_y + f32::from(self.data.line_height()) * scale
            },
        )
    }
}
