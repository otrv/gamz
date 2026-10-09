use std::time::Duration;

use platform_api::audio::{MAX_AUDIO_FRAMES, SampleRate};
use platform_api::input::{ButtonPosition, ButtonState, ControllerInput, FrameInput};
use platform_api::timing::DeltaTime;
use serde::{Deserialize, Deserializer, de::Error};

#[derive(Deserialize)]
#[serde(try_from = "InputFields")]
pub(super) enum Input {
    Frame(FrameInput),
    Audio {
        sample_rate: SampleRate,
        frames: usize,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InputFields {
    #[serde(default, deserialize_with = "present")]
    dt: Option<Elapsed>,
    #[serde(default, deserialize_with = "present")]
    controller: Option<Controller>,
    #[serde(default, deserialize_with = "present")]
    audio: Option<AudioFields>,
}

fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}

#[derive(Deserialize)]
struct Elapsed(#[serde(deserialize_with = "delta_time")] DeltaTime);

#[derive(Deserialize)]
struct Controller(#[serde(with = "ControllerFields")] ControllerInput);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AudioFields {
    sample_rate: u32,
    frames: usize,
}

impl TryFrom<InputFields> for Input {
    type Error = &'static str;

    fn try_from(fields: InputFields) -> Result<Self, Self::Error> {
        match (fields.dt, fields.controller, fields.audio) {
            (Some(Elapsed(dt)), Some(Controller(controller)), None) => {
                Ok(Self::Frame(FrameInput { dt, controller }))
            }
            (None, None, Some(audio)) => {
                let sample_rate = SampleRate::from_hz(audio.sample_rate)
                    .ok_or("audio.sample_rate must be between 1 and 192000")?;
                if audio.frames > MAX_AUDIO_FRAMES {
                    return Err("audio.frames exceeds 16384");
                }
                Ok(Self::Audio {
                    sample_rate,
                    frames: audio.frames,
                })
            }
            (_, _, Some(_)) => Err("audio cannot be combined with dt or controller"),
            (None, _, None) => Err("missing field dt"),
            (_, None, None) => Err("missing field controller"),
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(remote = "ControllerInput", default, deny_unknown_fields)]
struct ControllerFields {
    #[serde(deserialize_with = "button")]
    move_up: ButtonState,
    #[serde(deserialize_with = "button")]
    move_down: ButtonState,
    #[serde(deserialize_with = "button")]
    move_left: ButtonState,
    #[serde(deserialize_with = "button")]
    move_right: ButtonState,
    #[serde(deserialize_with = "button")]
    action_up: ButtonState,
    #[serde(deserialize_with = "button")]
    action_down: ButtonState,
    #[serde(deserialize_with = "button")]
    action_left: ButtonState,
    #[serde(deserialize_with = "button")]
    action_right: ButtonState,
    #[serde(deserialize_with = "button")]
    left_shoulder: ButtonState,
    #[serde(deserialize_with = "button")]
    right_shoulder: ButtonState,
    #[serde(deserialize_with = "button")]
    back: ButtonState,
    #[serde(deserialize_with = "button")]
    start: ButtonState,
}

#[derive(Deserialize)]
#[serde(remote = "ButtonPosition", rename_all = "snake_case")]
enum Position {
    Up,
    Down,
}

fn button<'de, D: Deserializer<'de>>(deserializer: D) -> Result<ButtonState, D::Error> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Fields {
        #[serde(with = "Position")]
        ended: ButtonPosition,
        half_transitions: u32,
    }

    let fields = Fields::deserialize(deserializer)?;
    Ok(ButtonState::new(fields.ended, fields.half_transitions))
}

fn delta_time<'de, D: Deserializer<'de>>(deserializer: D) -> Result<DeltaTime, D::Error> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Fields {
        secs: u64,
        nanos: u32,
    }

    let fields = Fields::deserialize(deserializer)?;
    if fields.nanos >= 1_000_000_000 {
        return Err(D::Error::custom("dt.nanos must be less than 1000000000"));
    }
    Ok(DeltaTime::from_elapsed(Duration::new(
        fields.secs,
        fields.nanos,
    )))
}
