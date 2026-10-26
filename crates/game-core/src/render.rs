use platform_api::render::RenderCommand;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderError {
    CommandCapacityExceeded,
}

pub struct RenderCommands<'a> {
    storage: &'a mut [RenderCommand<'a>],
    len: usize,
}

impl<'a> RenderCommands<'a> {
    #[must_use]
    pub fn new(storage: &'a mut [RenderCommand<'a>]) -> Self {
        Self { storage, len: 0 }
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
        let slot = self
            .storage
            .get_mut(self.len)
            .ok_or(RenderError::CommandCapacityExceeded)?;
        *slot = command;
        self.len += 1;
        Ok(())
    }

    #[must_use]
    pub fn finish(self) -> &'a [RenderCommand<'a>] {
        &self.storage[..self.len]
    }
}

#[cfg(test)]
mod tests {
    use crate::render::{RenderCommands, RenderError};
    use platform_api::render::RenderCommand;

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
}
