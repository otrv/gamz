use crate::font::{Font, MAX_TEXT_BYTES};
use platform_api::render::{
    Color, GlyphQuad, MAX_COMMANDS, MAX_QUADS, Point, RenderCommand, Sampling, Scale, Text,
};

pub struct RenderCommands<'a> {
    storage: &'a mut [RenderCommand<'a>],
    glyphs: &'a mut [GlyphQuad],
    len: usize,
}

impl<'a> RenderCommands<'a> {
    #[must_use]
    pub fn new(storage: &'a mut [RenderCommand<'a>], glyphs: &'a mut [GlyphQuad]) -> Self {
        assert!(storage.len() <= MAX_COMMANDS && glyphs.len() <= MAX_QUADS);
        Self {
            storage,
            glyphs,
            len: 0,
        }
    }

    pub fn push(&mut self, command: RenderCommand<'a>) {
        assert!(
            self.len < self.storage.len(),
            "render command budget exhausted"
        );
        self.storage[self.len] = command;
        self.len += 1;
    }

    pub fn draw_text(
        &mut self,
        font: &Font,
        content: &str,
        position: Point,
        scale: Scale,
        color: Color,
        sampling: Sampling,
    ) {
        assert!(content.len() <= MAX_TEXT_BYTES);
        assert!(content.len() <= self.glyphs.len(), "glyph budget exhausted");
        let (output, remaining) = core::mem::take(&mut self.glyphs).split_at_mut(content.len());
        self.glyphs = remaining;
        let glyphs = font.prepare(content, position, scale, output);
        self.push(RenderCommand::DrawText(Text::new(
            glyphs,
            font.texture(),
            color,
            sampling,
        )));
    }

    #[must_use]
    pub fn finish(self) -> &'a [RenderCommand<'a>] {
        &self.storage[..self.len]
    }
}
