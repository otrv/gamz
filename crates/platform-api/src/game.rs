use crate::Platform;
use crate::audio::AudioBuffer;
use crate::input::FrameInput;
use crate::memory::{PersistentMemory, TransientMemory};
use crate::render::FrameOutput;
use crate::services::StartupError;

pub trait Game: Sized {
    fn initialize<'p>(
        memory: PersistentMemory<'p>,
        transient_capacity: usize,
        platform: &mut impl Platform<'p>,
    ) -> Result<&'p mut Self, StartupError>;

    fn update<'p, 't>(
        &mut self,
        platform: &mut impl Platform<'p>,
        memory: TransientMemory<'t>,
        input: &FrameInput<'_>,
    ) -> FrameOutput<'t>;

    fn update_audio<'p>(&mut self, platform: &mut impl Platform<'p>, buffer: AudioBuffer<'_>);
}
