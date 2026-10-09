use std::time::Duration;

use platform_api::input::{ButtonPosition, ButtonState, ControllerInput, FrameInput};
use platform_api::timing::DeltaTime;
use serde::{Deserialize, Deserializer, de::Error};

#[derive(Deserialize)]
pub(super) struct Input(#[serde(with = "InputFields")] pub(super) FrameInput);

#[derive(Deserialize)]
#[serde(remote = "FrameInput", deny_unknown_fields)]
struct InputFields {
    #[serde(deserialize_with = "delta_time")]
    dt: DeltaTime,
    #[serde(with = "ControllerFields")]
    controller: ControllerInput,
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
