pub const MAX_AUDIO_FRAMES: usize = 16_384;
const MAX_SAMPLE_RATE_HZ: u32 = 192_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SampleRate(u32);

impl SampleRate {
    #[must_use]
    pub fn from_hz(hz: u32) -> Option<Self> {
        (1..=MAX_SAMPLE_RATE_HZ).contains(&hz).then_some(Self(hz))
    }

    #[must_use]
    pub fn hz(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StereoFrame {
    pub left: i16,
    pub right: i16,
}

pub struct AudioBuffer<'a> {
    sample_rate: SampleRate,
    frames: &'a mut [StereoFrame],
}

impl<'a> AudioBuffer<'a> {
    #[must_use]
    pub fn new(sample_rate: SampleRate, frames: &'a mut [StereoFrame]) -> Self {
        assert!(frames.len() <= MAX_AUDIO_FRAMES);
        Self {
            sample_rate,
            frames,
        }
    }

    #[must_use]
    pub fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    pub fn frames_mut(&mut self) -> &mut [StereoFrame] {
        self.frames
    }
}
