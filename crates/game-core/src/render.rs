use platform_api::render::{MAX_COMMANDS, MAX_QUADS, RenderCommand};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderError {
    CommandCapacityExceeded,
    QuadBudgetExceeded,
}

pub struct RenderCommands<'a> {
    storage: &'a mut [RenderCommand<'a>],
    len: usize,
    quads: usize,
}

impl<'a> RenderCommands<'a> {
    #[must_use]
    pub fn new(storage: &'a mut [RenderCommand<'a>]) -> Self {
        assert!(storage.len() <= MAX_COMMANDS);
        Self {
            storage,
            len: 0,
            quads: 0,
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[must_use]
    pub fn capacity(&self) -> usize {
        self.storage.len()
    }

    pub fn push(&mut self, command: RenderCommand<'a>) -> Result<(), RenderError> {
        if self.len == self.storage.len() {
            return Err(RenderError::CommandCapacityExceeded);
        }
        let quads = self.quads + command.quad_count();
        if quads > MAX_QUADS {
            return Err(RenderError::QuadBudgetExceeded);
        }
        self.storage[self.len] = command;
        self.len += 1;
        self.quads = quads;
        Ok(())
    }

    #[must_use]
    pub fn finish(self) -> &'a [RenderCommand<'a>] {
        &self.storage[..self.len]
    }
}

#[cfg(test)]
mod tests {
    use super::{RenderCommands, RenderError};
    use math::Radians;
    use platform_api::render::{
        Canvas, Color, Frame, GlyphQuad, MAX_QUADS, PixelRect, Point, Rect, RectStyle,
        RenderCommand, Sampling, Size, StrokeWidth, Text, TextureId, Transform,
    };

    #[test]
    fn exhausted_command_storage_preserves_accepted_commands() {
        let mut storage = [RenderCommand::SetClip(None); 1];
        let mut commands = RenderCommands::new(&mut storage);
        assert!(commands.is_empty());
        assert_eq!(commands.capacity(), 1);
        commands.push(RenderCommand::SetClip(None)).unwrap();
        assert_eq!(
            commands.push(RenderCommand::SetClip(None)),
            Err(RenderError::CommandCapacityExceeded)
        );
        assert_eq!(commands.len(), 1);
        assert!(matches!(commands.finish(), [RenderCommand::SetClip(None)]));
        let mut empty = RenderCommands::new(&mut []);
        assert_eq!(
            empty.push(RenderCommand::SetClip(None)),
            Err(RenderError::CommandCapacityExceeded)
        );
        assert!(empty.finish().is_empty());
    }

    #[test]
    fn mixed_quad_budget_rejects_atomically_and_accepts_exact_fit() {
        let glyph = GlyphQuad {
            source: PixelRect::new(0, 0, 1, 1),
            transform: Transform {
                center: Point::new(1.0, 2.0),
                size: Size::new(3.0, 4.0),
                rotation: Radians::new(0.0),
            },
        };
        let glyphs = [glyph; 512];
        let text = Text::new(&glyphs, TextureId::new(0), Color::WHITE, Sampling::Nearest);
        let mut storage = [RenderCommand::SetClip(None); 134];
        let mut commands = RenderCommands::new(&mut storage);
        for _ in 0..127 {
            commands.push(RenderCommand::DrawText(text)).unwrap();
        }
        let outline = RenderCommand::DrawRect {
            rect: Rect::new(Point::new(0.0, 0.0), Size::new(10.0, 20.0)),
            style: RectStyle::Outline {
                color: Color::WHITE,
                width: StrokeWidth::new(1.0),
            },
        };
        commands.push(outline).unwrap();
        assert_eq!(
            commands.push(RenderCommand::DrawText(text)),
            Err(RenderError::QuadBudgetExceeded)
        );
        assert_eq!(commands.len(), 128);
        commands
            .push(RenderCommand::DrawText(Text::new(
                &glyphs[..508],
                TextureId::new(1),
                Color::WHITE,
                Sampling::Linear,
            )))
            .unwrap();
        assert_eq!(commands.push(outline), Err(RenderError::QuadBudgetExceeded));
        commands.push(RenderCommand::SetClip(None)).unwrap();
        let frame = Frame::new(Canvas::new(100, 100), Color::WHITE, commands.finish());
        assert_eq!(frame.commands().len(), 130);
        let quads: usize = frame
            .commands()
            .iter()
            .map(|command| command.quad_count())
            .sum();
        assert_eq!(quads, MAX_QUADS);
    }
}
