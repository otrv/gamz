use crate::Platform;
use crate::audio::StereoSample;
use crate::input::Input;
use crate::memory::{PersistentMemory, TransientMemory};
use crate::render::{Frame, TextureError};

pub trait Game: Sized {
    fn initialize<'persistent, Context>(
        memory: PersistentMemory<'persistent>,
        transient_capacity: usize,
        platform: &mut Platform<'_, Context>,
    ) -> Result<&'persistent mut Self, StartupError>;

    fn update<'frame, Context>(
        &mut self,
        input: &Input<'_>,
        memory: TransientMemory<'frame>,
        platform: &mut Platform<'_, Context>,
    ) -> Result<Frame<'frame>, FrameError>;

    fn update_audio(&mut self, sample_rate_hz: u32, samples: &mut [StereoSample]);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartupError {
    InsufficientPersistentMemory { required: usize, available: usize },
    InsufficientTransientMemory { required: usize, available: usize },
    Texture(TextureError),
    Game { code: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameError {
    InsufficientTransientMemory { required: usize, available: usize },
    Texture(TextureError),
}
