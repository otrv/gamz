use std::ops::Range;

use alsa::pcm::{Access, Format, Frames, HwParams, PCM, State};
use alsa::{Direction, ValueOr};
use platform_api::audio::{AudioBuffer, MAX_AUDIO_FRAMES, SampleRate, StereoFrame};
use rustix::io::Errno;

const BYTES_PER_FRAME: usize = 4;
const TARGET_QUEUE_MILLISECONDS: u32 = 50;

pub(super) struct Audio {
    pcm: PCM,
    device_frames: Frames,
    sample_rate: SampleRate,
    frames: Vec<StereoFrame>,
    encoded: Vec<u8>,
    pending: Range<usize>,
}

impl Audio {
    pub(super) fn start(state: &mut game::GameState) -> Result<Self, Box<dyn std::error::Error>> {
        let pcm = PCM::open(c"default", Direction::Playback, true)?;
        let (sample_rate, target, device_frames) = {
            let parameters = HwParams::any(&pcm)?;
            parameters.set_access(Access::RWInterleaved)?;
            parameters.set_format(Format::s16())?;
            parameters.set_channels(2)?;
            parameters.set_rate_near(48_000, ValueOr::Nearest)?;
            parameters.set_period_time_near(10_000, ValueOr::Nearest)?;
            parameters.set_buffer_time_near(100_000, ValueOr::Nearest)?;
            pcm.hw_params(&parameters)?;
            let actual = pcm.hw_params_current()?;
            let sample_rate = SampleRate::from_hz(actual.get_rate()?)
                .ok_or("audio sample rate exceeds the supported range")?;
            let target =
                usize::try_from((sample_rate.hz() * TARGET_QUEUE_MILLISECONDS).div_ceil(1000))?;
            let buffer_size = actual.get_buffer_size()?;
            if target > MAX_AUDIO_FRAMES
                || buffer_size < Frames::try_from(target)?
                || buffer_size > Frames::from(sample_rate.hz()) / 2
                || actual.get_period_size()? > Frames::try_from(target)?
            {
                return Err("audio device cannot satisfy the queue budget".into());
            }
            let software = pcm.sw_params_current()?;
            software.set_start_threshold(Frames::try_from(target)?)?;
            software.set_stop_threshold(buffer_size)?;
            pcm.sw_params(&software)?;
            (sample_rate, target, buffer_size)
        };
        let mut frames = Vec::new();
        frames.try_reserve_exact(target)?;
        frames.resize(target, StereoFrame::default());
        let mut encoded = Vec::new();
        encoded.try_reserve_exact(target * BYTES_PER_FRAME)?;
        encoded.resize(target * BYTES_PER_FRAME, 0);
        pcm.prepare()?;
        let mut audio = Self {
            pcm,
            device_frames,
            sample_rate,
            frames,
            encoded,
            pending: 0..0,
        };
        audio.update(state)?;
        Ok(audio)
    }

    pub(super) fn update(&mut self, state: &mut game::GameState) -> alsa::Result<()> {
        match self.fill(state) {
            Err(error) if error.errno() == Errno::PIPE.raw_os_error() => {
                self.pcm.prepare()?;
                self.pending = 0..0;
                self.fill(state)
            }
            result => result,
        }
    }

    fn fill(&mut self, state: &mut game::GameState) -> alsa::Result<()> {
        if !self.pending.is_empty() {
            self.write_pending()?;
            if !self.pending.is_empty() {
                return Ok(());
            }
        }
        let available = if self.pcm.state() == State::Prepared {
            self.pcm.avail_update()?
        } else {
            self.pcm.avail()?
        }
        .clamp(0, self.device_frames);
        let queued = self.device_frames - available;
        let target = Frames::try_from(self.frames.len()).unwrap();
        let requested = (target - queued).max(0).min(available);
        let requested = usize::try_from(requested).unwrap();
        if requested == 0 {
            return Ok(());
        }
        let frames = &mut self.frames[..requested];
        game::update_audio(state, AudioBuffer::new(self.sample_rate, frames));
        for (frame, bytes) in frames
            .iter()
            .zip(self.encoded.as_chunks_mut::<BYTES_PER_FRAME>().0)
        {
            bytes[..2].copy_from_slice(&frame.left.to_ne_bytes());
            bytes[2..].copy_from_slice(&frame.right.to_ne_bytes());
        }
        self.pending = 0..requested;
        self.write_pending()
    }

    fn write_pending(&mut self) -> alsa::Result<()> {
        let bytes =
            &self.encoded[self.pending.start * BYTES_PER_FRAME..self.pending.end * BYTES_PER_FRAME];
        match self.pcm.io_bytes().writei(bytes) {
            Ok(written) => {
                assert!(written <= self.pending.len());
                self.pending.start += written;
                Ok(())
            }
            Err(error) if error.errno() == Errno::AGAIN.raw_os_error() => Ok(()),
            Err(error) => Err(error),
        }
    }
}
