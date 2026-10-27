use std::collections::TryReserveError;
use std::mem::{MaybeUninit, align_of, size_of};

use ::memory::Arena;
use platform_api::render::{RenderCommand, RenderCommands};

use crate::limits::MAX_COMMANDS;

pub(crate) const PERSISTENT_MEMORY_BYTES: usize = 64 * 1024;
pub(crate) const TRANSIENT_MEMORY_BYTES: usize = 256 * 1024 * 1024;
pub(crate) const COMMAND_MEMORY_BYTES: usize =
    size_of::<RenderCommand<'static>>() * MAX_COMMANDS + align_of::<RenderCommand<'static>>() - 1;

pub(crate) fn allocate(size: usize) -> Result<Vec<u8>, TryReserveError> {
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(size)?;
    Ok(bytes)
}

pub(crate) fn commands(bytes: &mut [MaybeUninit<u8>]) -> RenderCommands<'_> {
    RenderCommands {
        storage: Arena::new(bytes).push_slice(MAX_COMMANDS, RenderCommand::SetClip(None)),
        len: 0,
    }
}
