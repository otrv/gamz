#![no_std]

use game_memory::Arena;
use platform_api::audio::{AudioBuffer, StereoFrame};
use platform_api::input::FrameInput;
use platform_api::memory::{PersistentMemory, TransientMemory};
use platform_api::render::{Canvas, Color, Frame, TextureUpload};
use platform_api::services::{PlatformApi, StartupError};

pub struct GameState(u8);

const MINIMUM_PERSISTENT_MEMORY_BYTES: usize = core::mem::size_of::<GameState>();

pub fn initialize<'p, 't>(
    persistent: PersistentMemory<'p>,
    _transient: TransientMemory<'t>,
    _api: &PlatformApi,
) -> Result<(&'p mut GameState, [TextureUpload<'t>; 0]), StartupError> {
    let PersistentMemory { bytes: persistent } = persistent;
    if persistent.len() < MINIMUM_PERSISTENT_MEMORY_BYTES {
        return Err(StartupError::InsufficientPersistentMemory {
            required: MINIMUM_PERSISTENT_MEMORY_BYTES,
            available: persistent.len(),
        });
    }
    let mut persistent = Arena::new(persistent);
    Ok((persistent.push(GameState(0)), []))
}

pub fn update<'a>(
    state: &mut GameState,
    _memory: TransientMemory<'a>,
    _input: &FrameInput,
) -> Frame<'a> {
    debug_assert_eq!(state.0, 0);
    Frame {
        canvas: Canvas::new(1280, 720),
        clear: Color(0, 0, 0, 255),
        commands: &[],
    }
}

pub fn update_audio(_state: &mut GameState, mut buffer: AudioBuffer<'_>) {
    buffer.frames_mut().fill(StereoFrame::default());
}
