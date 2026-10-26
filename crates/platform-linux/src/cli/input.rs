use std::time::Duration;

use math::Vec2;
use platform_api::input::{Actions, ButtonPosition, ButtonState};
use serde::de::Error;
use serde::{Deserialize, Deserializer};

use crate::controls::released_button;
use crate::limits::{MAX_AUDIO_FRAMES, MAX_SAMPLE_RATE_HZ, MAX_TEXT_BYTES};

#[derive(Deserialize)]
#[serde(try_from = "InputFields")]
pub(super) enum Input {
    Frame {
        dt: Duration,
        controller: Actions,
        close_requested: bool,
        pointer: Option<Vec2>,
        scroll: Vec2,
        text: String,
    },
    Audio {
        sample_rate: u32,
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
    #[serde(default, deserialize_with = "present")]
    close_requested: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    pointer: Option<Vector>,
    #[serde(default, deserialize_with = "present")]
    scroll: Option<Vector>,
    #[serde(default, deserialize_with = "present")]
    text: Option<String>,
}

fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}

#[derive(Deserialize)]
struct Elapsed(#[serde(deserialize_with = "delta_time")] Duration);

#[derive(Deserialize)]
struct Controller(#[serde(with = "ControllerFields")] Actions);

#[derive(Deserialize)]
struct Vector(#[serde(deserialize_with = "finite_vector")] Vec2);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AudioFields {
    sample_rate: u32,
    frames: usize,
}

impl TryFrom<InputFields> for Input {
    type Error = &'static str;

    fn try_from(fields: InputFields) -> Result<Self, Self::Error> {
        if let Some(audio) = fields.audio {
            if fields.dt.is_some()
                || fields.controller.is_some()
                || fields.close_requested.is_some()
                || fields.pointer.is_some()
                || fields.scroll.is_some()
                || fields.text.is_some()
            {
                return Err(
                    "audio cannot be combined with dt, controller, close_requested, pointer, scroll or text",
                );
            }
            if audio.sample_rate == 0 || audio.sample_rate > MAX_SAMPLE_RATE_HZ {
                return Err("audio.sample_rate must be between 1 and 192000");
            }
            if audio.frames > MAX_AUDIO_FRAMES {
                return Err("audio.frames exceeds 16384");
            }
            return Ok(Self::Audio {
                sample_rate: audio.sample_rate,
                frames: audio.frames,
            });
        }

        let Elapsed(dt) = fields.dt.ok_or("missing field dt")?;
        let Controller(controller) = fields.controller.ok_or("missing field controller")?;
        let text = fields.text.unwrap_or_default();
        if text.len() > MAX_TEXT_BYTES {
            return Err("text exceeds byte limit");
        }
        Ok(Self::Frame {
            dt,
            controller,
            close_requested: fields.close_requested.unwrap_or(false),
            pointer: fields.pointer.map(|Vector(vector)| vector),
            scroll: fields.scroll.map_or(Vec2::ZERO, |Vector(vector)| vector),
            text,
        })
    }
}

#[derive(Deserialize)]
#[serde(remote = "Actions", deny_unknown_fields)]
struct ControllerFields {
    #[serde(default, deserialize_with = "unit_vector")]
    movement: Vec2,
    #[serde(default, deserialize_with = "unit_vector")]
    aim: Vec2,
    #[serde(default = "released_button", deserialize_with = "button")]
    move_up: ButtonState,
    #[serde(default = "released_button", deserialize_with = "button")]
    move_down: ButtonState,
    #[serde(default = "released_button", deserialize_with = "button")]
    move_left: ButtonState,
    #[serde(default = "released_button", deserialize_with = "button")]
    move_right: ButtonState,
    #[serde(default = "released_button", deserialize_with = "button")]
    action_up: ButtonState,
    #[serde(default = "released_button", deserialize_with = "button")]
    action_down: ButtonState,
    #[serde(default = "released_button", deserialize_with = "button")]
    action_left: ButtonState,
    #[serde(default = "released_button", deserialize_with = "button")]
    action_right: ButtonState,
    #[serde(default = "released_button", deserialize_with = "button")]
    left_shoulder: ButtonState,
    #[serde(default = "released_button", deserialize_with = "button")]
    right_shoulder: ButtonState,
    #[serde(default = "released_button", deserialize_with = "button")]
    back: ButtonState,
    #[serde(default = "released_button", deserialize_with = "button")]
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
    Ok(ButtonState {
        ended: fields.ended,
        half_transitions: fields.half_transitions,
    })
}

fn finite_vector<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec2, D::Error> {
    let [x, y] = <[f32; 2]>::deserialize(deserializer)?;
    if !x.is_finite() || !y.is_finite() {
        return Err(D::Error::custom("vector components must be finite"));
    }
    Ok(Vec2 { x, y })
}

fn unit_vector<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec2, D::Error> {
    let vector = finite_vector(deserializer)?;
    if !(-1.0..=1.0).contains(&vector.x) || !(-1.0..=1.0).contains(&vector.y) {
        return Err(D::Error::custom(
            "controller vector components must be between -1 and 1",
        ));
    }
    Ok(vector)
}

fn delta_time<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
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
    Ok(Duration::new(fields.secs, fields.nanos))
}
