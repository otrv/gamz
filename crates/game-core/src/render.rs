use platform_api::render::{RenderCommand, RenderCommands};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderError {
    CommandCapacityExceeded,
}

pub fn push<'frame>(
    commands: &mut RenderCommands<'frame>,
    command: RenderCommand<'frame>,
) -> Result<(), RenderError> {
    let slot = commands
        .storage
        .get_mut(commands.len)
        .ok_or(RenderError::CommandCapacityExceeded)?;
    *slot = command;
    commands.len += 1;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::render::{self, RenderError};
    use math::Vec2;
    use platform_api::render::{RenderCommand, RenderCommands};

    #[test]
    fn exhausted_command_storage_preserves_accepted_commands() {
        let mut storage = [RenderCommand::SetClip(None); 2];
        let mut commands = RenderCommands {
            storage: &mut storage,
            len: 0,
        };
        render::push(
            &mut commands,
            RenderCommand::SetCamera {
                origin: Vec2::new(3.0, 7.0),
                zoom: 2.0,
            },
        )
        .unwrap();
        render::push(&mut commands, RenderCommand::SetClip(None)).unwrap();
        assert_eq!(
            render::push(&mut commands, RenderCommand::SetClip(None)),
            Err(RenderError::CommandCapacityExceeded)
        );
        assert_eq!(commands.len, 2);
        let [
            RenderCommand::SetCamera { origin, zoom },
            RenderCommand::SetClip(None),
        ] = &commands.storage[..commands.len]
        else {
            panic!("accepted commands changed after exhaustion");
        };
        assert_eq!(*origin, Vec2::new(3.0, 7.0));
        assert_eq!(*zoom, 2.0);
        let mut empty = RenderCommands {
            storage: &mut [],
            len: 0,
        };
        assert_eq!(
            render::push(&mut empty, RenderCommand::SetClip(None)),
            Err(RenderError::CommandCapacityExceeded)
        );
        assert_eq!(empty.len, 0);
    }
}
